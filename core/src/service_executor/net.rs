use super::runtime::get_user_agent;
use super::{HttpJob, ServiceCommand, runtime};
use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http::{HeaderValue, header::HeaderName};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper_util::client::legacy::Client;
use std::io::{Error, ErrorKind};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

pub const DEFAULT_BLOCKING_BODY_LIMIT: usize = 512 * 1024;

pub async fn post_json(
    url: &str,
    body: &[u8],
    extra_headers: Option<&[(&str, &str)]>,
) -> Result<(http::StatusCode, Bytes), String> {
    let mut builder = HttpRequest::builder()
        .method("POST")
        .uri(url)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json");

    if let Some(h) = builder.headers_mut() {
        let ua = get_user_agent();
        let ua_val =
            HeaderValue::from_str(&ua).map_err(|e| format!("invalid user agent header: {}", e))?;
        h.insert(header::USER_AGENT, ua_val);

        if let Some(extras) = extra_headers {
            for (key, value) in extras {
                let name = HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| format!("invalid header name '{}': {}", key, e))?;
                let val = HeaderValue::from_str(value)
                    .map_err(|e| format!("invalid header '{}' value: {}", key, e))?;
                h.insert(name, val);
            }
        }
    }

    let body_bytes = Bytes::copy_from_slice(body);
    let request_body: BoxBody<Bytes, Error> = Full::new(body_bytes)
        .map_err(|_| Error::new(ErrorKind::Other, "body error"))
        .boxed();

    let request = builder
        .body(request_body)
        .map_err(|e| format!("build request: {}", e))?;

    let response = send_request(request, DEFAULT_BLOCKING_BODY_LIMIT, None).await?;
    let status = response.status;
    let bytes = collect_body_bytes(response.body).await?;
    Ok((status, bytes))
}

pub async fn send_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, String> {
    let tx = {
        let guard = runtime::runtime_slot().lock().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.tx.clone()
        } else {
            return Err("service runtime not started".to_string());
        }
    };
    let (resp_tx, resp_rx) = oneshot::channel();
    let msg = HttpJob {
        request,
        small_threshold,
        resp_tx,
        abort_rx,
    };
    tx.send(ServiceCommand::Http(msg))
        .await
        .map_err(|e| format!("service runtime down: {}", e))?;
    resp_rx
        .await
        .map_err(|e| format!("service response dropped: {}", e))?
}

pub(crate) async fn process_request(
    client: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        BoxBody<Bytes, Error>,
    >,
    mut msg: HttpJob,
) {
    let req = msg.request;
    let small = msg.small_threshold;
    const READ_FRAME_TIMEOUT: Duration = Duration::from_secs(120);

    let mut abort_for_request = msg.abort_rx.as_mut();
    let resp = if let Some(abort_rx) = &mut abort_for_request {
        tokio::select! {
            res = client.request(req) => {
                match res {
                    Ok(r) => r,
                    Err(e) => { let _ = msg.resp_tx.send(Err(format!("request failed: {}", e))); return; }
                }
            }
            _ = abort_rx => {
                let _ = msg.resp_tx.send(Err("aborted".to_string()));
                return;
            }
        }
    } else {
        match client.request(req).await {
            Ok(r) => r,
            Err(e) => {
                let _ = msg.resp_tx.send(Err(format!("request failed: {}", e)));
                return;
            }
        }
    };
    let (parts, mut body) = resp.into_parts();

    let cl = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    if cl > 0 && cl <= small {
        let mut buf = Vec::with_capacity(cl);
        let has_abort = msg.abort_rx.is_some();
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Some(data) = frame.data_ref() { buf.extend_from_slice(data); }
                                if buf.len() > small { let _ = msg.resp_tx.send(Err("body exceeded small threshold".to_string())); return; }
                            }
                            Ok(Some(Err(e))) => { let _ = msg.resp_tx.send(Err(format!("read frame: {}", e))); return; }
                            Ok(None) => break,
                            Err(_) => { let _ = msg.resp_tx.send(Err("read timeout".to_string())); return; }
                        }
                    }
                    _ = async { if let Some(rx) = &mut msg.abort_rx { let _ = rx.await; } } => {
                        let _ = msg.resp_tx.send(Err("aborted".to_string()));
                        return;
                    }
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Some(data) = frame.data_ref() {
                            buf.extend_from_slice(data);
                        }
                        if buf.len() > small {
                            let _ = msg
                                .resp_tx
                                .send(Err("body exceeded small threshold".to_string()));
                            return;
                        }
                    }
                    Ok(Some(Err(e))) => {
                        let _ = msg.resp_tx.send(Err(format!("read frame: {}", e)));
                        return;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        let _ = msg.resp_tx.send(Err("read timeout".to_string()));
                        return;
                    }
                }
            }
        }
        let _ = msg.resp_tx.send(Ok(HttpResponse {
            status: parts.status,
            headers: parts.headers,
            body: HttpBody::Small(Bytes::from(buf)),
        }));
        return;
    }

    let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(8);
    let mut abort = msg.abort_rx.take();
    tokio::task::spawn(async move {
        let mut body = body;
        let has_abort = abort.is_some();
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Ok(data) = frame.into_data() { if tx.send(Ok(data)).await.is_err() { break; } }
                            }
                            Ok(Some(Err(e))) => { let _ = tx.send(Err(format!("read frame: {}", e))).await; break; }
                            Ok(None) => break,
                            Err(_) => { let _ = tx.send(Err("read timeout".to_string())).await; break; }
                        }
                    }
                    _ = async { if let Some(rx) = &mut abort { let _ = rx.await; } } => { let _ = tx.send(Err("aborted".to_string())).await; drop(tx); break; }
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Ok(data) = frame.into_data() {
                            if tx.send(Ok(data)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        let _ = tx.send(Err(format!("read frame: {}", e))).await;
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        let _ = tx.send(Err("read timeout".to_string())).await;
                        break;
                    }
                }
            }
        }
    });
    let _ = msg.resp_tx.send(Ok(HttpResponse {
        status: parts.status,
        headers: parts.headers,
        body: HttpBody::Stream(rx),
    }));
}

pub struct HttpResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: HttpBody,
}

pub enum HttpBody {
    Empty,
    Small(Bytes),
    Stream(mpsc::Receiver<Result<Bytes, String>>),
}

async fn collect_body_bytes(body: HttpBody) -> Result<Bytes, String> {
    match body {
        HttpBody::Empty => Ok(Bytes::new()),
        HttpBody::Small(bytes) => Ok(bytes),
        HttpBody::Stream(mut rx) => {
            let mut buf = Vec::new();
            while let Some(chunk_res) = rx.recv().await {
                let chunk = chunk_res?;
                buf.extend_from_slice(&chunk);
            }
            Ok(Bytes::from(buf))
        }
    }
}
