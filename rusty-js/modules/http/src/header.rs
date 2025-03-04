use http_crate::header::{self, HeaderMap, HeaderName, HeaderValue};
use rusty_js::{
    function::{Optional, This},
    js_export, js_method, js_methods, *,
};

#[js_export]
#[derive(Default)]
pub struct Headers {
    headers: HeaderMap<HeaderValue>,
}

#[js_methods]
impl Headers {
    #[js_method(constructor)]
    pub fn new(init: Optional<JSValue>) -> JSResult<Self> {
        let mut headers = HeaderMap::new();

        if let Some(init) = init.0 {
            if let Some(obj) = init.into_object() {
                // instance of Headers
                if let Ok(other_headers) = obj.borrow::<Headers>() {
                    headers.extend(
                        other_headers
                            .headers
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone())),
                    );
                } else if let Some(array) = JSArray::from_object(obj.clone()) {
                    for item in array.iter::<JSValue>() {
                        let item = item?;
                        if let Some(pair) = item.into_object().and_then(JSArray::from_object) {
                            if pair.len() != 2 {
                                return Err(RustyJSError::TypeError(
                                    "Each header must be an array of [name, value]".to_string(),
                                ));
                            }

                            let key: String = pair.get(0)?.ok_or_else(|| {
                                RustyJSError::TypeError("Header name is required".to_string())
                            })?;
                            let value: String = pair.get(1)?.ok_or_else(|| {
                                RustyJSError::TypeError("Header value is required".to_string())
                            })?;

                            match (
                                HeaderName::try_from(key.as_str()),
                                HeaderValue::try_from(value.as_str()),
                            ) {
                                (Ok(name), Ok(value)) => {
                                    headers.append(name, value);
                                }
                                (Err(_), _) => {
                                    return Err(RustyJSError::TypeError(format!(
                                        "Invalid header name: {}",
                                        key
                                    )));
                                }
                                (_, Err(_)) => {
                                    return Err(RustyJSError::TypeError(
                                        "Invalid header value".to_string(),
                                    ));
                                }
                            }
                        } else {
                            return Err(RustyJSError::TypeError(
                                "Each header must be an array of [name, value]".to_string(),
                            ));
                        }
                    }
                } else {
                    for entry in obj.entries()? {
                        let (key, value) = entry.try_into::<String, String>()?;

                        match (
                            HeaderName::try_from(key.as_str()),
                            HeaderValue::try_from(value.as_str()),
                        ) {
                            (Ok(name), Ok(value)) => {
                                headers.append(name, value);
                            }
                            (Err(_), _) => {
                                return Err(RustyJSError::TypeError(format!(
                                    "Invalid header name: {}",
                                    key
                                )));
                            }
                            (_, Err(_)) => {
                                return Err(RustyJSError::TypeError(
                                    "Invalid header value".to_string(),
                                ));
                            }
                        }
                    }
                }
            } else {
                return Err(RustyJSError::TypeError("Invalid Headers init".to_string()));
            }
        }

        Ok(Self { headers })
    }

    #[js_method]
    pub fn append(&mut self, name: String, value: String) -> JSResult<()> {
        match (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            (Ok(name), Ok(value)) => {
                self.headers.append(name, value);
                Ok(())
            }
            (Err(_), _) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
            (_, Err(_)) => Err(RustyJSError::TypeError("Invalid header value".to_string())),
        }
    }

    #[js_method]
    pub fn delete(&mut self, name: String) -> JSResult<()> {
        match HeaderName::try_from(name.as_str()) {
            Ok(name) => {
                self.headers.remove(&name);
                Ok(())
            }
            Err(_) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
        }
    }

    #[js_method]
    pub fn get(&self, name: String) -> JSResult<String> {
        match HeaderName::try_from(name.as_str()) {
            Ok(name) => {
                let values: Vec<&str> = self
                    .headers
                    .get_all(&name)
                    .into_iter()
                    .filter_map(|v| v.to_str().ok())
                    .collect();

                if values.is_empty() {
                    return Err(RustyJSError::TypeError("Header not found".to_string()));
                }

                Ok(values.join(", "))
            }
            Err(_) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
        }
    }

    #[js_method]
    pub fn has(&self, name: String) -> JSResult<bool> {
        match HeaderName::try_from(name.as_str()) {
            Ok(name) => Ok(self.headers.contains_key(&name)),
            Err(_) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
        }
    }

    #[js_method]
    pub fn set(&mut self, name: String, value: String) -> JSResult<()> {
        // Check for null characters in value
        if value.contains('\0') {
            return Err(RustyJSError::TypeError(
                "Header value must not contain null characters".to_string(),
            ));
        }

        match (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            (Ok(name), Ok(value)) => {
                self.headers.insert(name, value);
                Ok(())
            }
            (Err(_), _) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
            (_, Err(_)) => Err(RustyJSError::TypeError("Invalid header value".to_string())),
        }
    }

    #[js_method]
    pub fn entries(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        for (i, (name, value)) in self.headers.iter().enumerate() {
            let entry = JSArray::new(&ctx)?;
            entry
                .set(0, name.as_str())?
                .set(1, value.to_str().unwrap_or_default())?;
            array.set(i as u32, entry)?;
        }
        Ok(array)
    }

    #[js_method]
    pub fn keys(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        for (i, name) in self.headers.keys().enumerate() {
            array.set(i as u32, name.as_str())?;
        }
        Ok(array)
    }

    #[js_method]
    pub fn values(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        for (i, value) in self.headers.values().enumerate() {
            array.set(i as u32, value.to_str().unwrap_or_default())?;
        }
        Ok(array)
    }

    #[js_method(rename = "getSetCookie")]
    pub fn get_set_cookie(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        let mut index = 0;

        // HeaderMap natively supports multi-value headers
        for cookie in self.headers.get_all(header::SET_COOKIE) {
            if let Ok(cookie_str) = cookie.to_str() {
                array.set(index, cookie_str)?;
                index += 1;
            }
        }
        Ok(array)
    }

    #[js_method(rename = "forEach")]
    pub fn for_each(
        &self,
        this: This<JSObject>, // Header Object
        callback: JSFunc,
        this_arg: Optional<JSObject>,
    ) -> JSResult<()> {
        for (name, value) in self.headers.iter() {
            let value_str = value.to_str().unwrap_or_default();
            let name_str = name.as_str();

            if let Some(ref this_arg) = this_arg.0 {
                callback.call_with_this::<_, ()>(
                    this_arg.clone(),
                    (value_str, name_str, this.0.clone()),
                )?;
            } else {
                callback.call::<_, ()>((value_str, name_str, this.0.clone()))?;
            }
        }
        Ok(())
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Headers>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_headers() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("JS: {}", msg)),
            )?;

            let source = Source::from_bytes(
                r#"
                    const console={
                        log: function(...args){
                            print(args.join(' '))
                        }
                    }
                "#,
            );
            ctx.eval::<()>(source)?;

            let source = Source::from_path("tests/headers.js").await.unwrap();
            let obj: JSObject = ctx.eval_async(source).await?;

            let total: i32 = obj.get("total").unwrap();
            let passed: i32 = obj.get("passed").unwrap();
            let success: bool = obj.get("success").unwrap();

            if !success {
                let failed: JSArray = obj.get("failed").unwrap();
                let error_messages: Vec<String> = failed.iter().collect::<JSResult<_>>()?;
                panic!(
                    "Headers tests failed:\nPassed {}/{}\nFailures:\n{}",
                    passed,
                    total,
                    error_messages.join("\n")
                );
            }
            Ok(())
        });
    }
}
