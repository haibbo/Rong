use std::ops::Deref;

use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use rusty_js::{
    function::{Optional, This},
    js_class, js_export, js_method, *,
};

#[js_export]
#[derive(Default)]
pub struct Headers {
    headers: HeaderMap<HeaderValue>,
}

impl Deref for Headers {
    type Target = HeaderMap;

    fn deref(&self) -> &Self::Target {
        &self.headers
    }
}

#[js_class]
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

    /// The append() method of the Headers interface appends a new value onto an
    /// existing header inside a Headers object, or adds the header if it does not
    /// already exist.
    #[js_method]
    pub fn append(&mut self, name: String, value: String) {
        if let (Ok(name), Ok(value)) = (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            self.headers.append(name, value);
        }
    }

    /// The delete() method of the Headers interface deletes a header from the current Headers object.
    #[js_method]
    pub fn delete(&mut self, name: String) {
        if let Ok(name) = HeaderName::try_from(name.as_str()) {
            self.headers.remove(&name);
        }
    }

    /// The get() method of the Headers interface returns a byte string of all the
    /// values of a header within a Headers object with a given name. If the requested
    /// header doesn't exist in the Headers object, it returns null.
    ///
    /// The name of the HTTP header whose values you want to retrieve from the Headers
    /// object. If the given name is not the name of an HTTP header, this method throws
    /// a TypeError. The name is case-insensitive.
    #[js_method]
    pub fn get(&self, name: String) -> JSResult<Option<String>> {
        match HeaderName::try_from(name.as_str()) {
            Ok(name) => {
                let values: Vec<&str> = self
                    .headers
                    .get_all(&name)
                    .into_iter()
                    .filter_map(|v| v.to_str().ok())
                    .collect();

                if values.is_empty() {
                    return Ok(None);
                }

                Ok(Some(values.join(", ")))
            }
            Err(_) => Err(RustyJSError::TypeError(format!(
                "Invalid header name: {}",
                name
            ))),
        }
    }

    /// The has() method  returns a boolean stating whether a Headers object contains
    /// a certain header.
    ///
    /// The name of the HTTP header you want to test for. If the given name is not a
    /// valid HTTP header name, this method throws a TypeError.
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

    /// The set() method sets a new value for an existing header inside a Headers
    /// object, or adds the header if it does not already exist.
    ///
    /// The name of the HTTP header you want to set to a new value. If the given
    /// name is not the name of an HTTP header, this method throws a TypeError.
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

    /// The Headers.entries() method returns an iterator allowing to go through all
    /// key/value pairs contained in this object. Both the key and value of each pair are String objects
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

    /// The Headers.keys() method returns an iterator allowing to go through all
    /// keys contained in this object. The keys are String objects.
    #[js_method]
    pub fn keys(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        for (i, name) in self.headers.keys().enumerate() {
            array.set(i as u32, name.as_str())?;
        }
        Ok(array)
    }

    /// The Headers.values() method returns an iterator allowing to go through all
    /// values contained in this object. The values are String objects
    #[js_method]
    pub fn values(&self, ctx: JSContext) -> JSResult<JSArray> {
        let array = JSArray::new(&ctx)?;
        for (i, value) in self.headers.values().enumerate() {
            array.set(i as u32, value.to_str().unwrap_or_default())?;
        }
        Ok(array)
    }

    /// getSetCookie() returns an array containing the values of all Set-Cookie
    /// headers associated with a response.
    ///
    /// If no Set-Cookie headers are set, the method will return an empty array
    #[js_method(rename = "getSetCookie")]
    pub fn get_set_cookie(&self) -> Vec<String> {
        let mut cookies = Vec::new();

        // HeaderMap natively supports multi-value headers
        for cookie in self.headers.get_all(header::SET_COOKIE) {
            if let Ok(cookie_str) = cookie.to_str() {
                cookies.push(cookie_str.to_string());
            }
        }
        cookies
    }

    /// forEach() method executes a callback function once per each key/value pair
    /// in the Headers object
    #[js_method(rename = "forEach")]
    pub fn for_each(
        &self,
        this: This<JSObject>, // Header Object
        callback: JSFunc,
        this_arg: Optional<JSObject>,
    ) {
        // Value to use as this when executing callback. It's optional
        let this_arg = this_arg.0;

        for (name, value) in self.headers.iter() {
            let value_str = value.to_str().unwrap_or_default();
            let name_str = name.as_str();

            let _ = callback.call::<_, ()>(this_arg.clone(), (value_str, name_str, this.0.clone()));
        }
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
            assert::init(&ctx)?;
            console::init(&ctx, None)?;

            let passed = UnitJSRunner::load_script(&ctx, "header.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }
}
