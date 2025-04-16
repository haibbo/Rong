use buffer::{Blob, File};
use rong_js::{function::*, *};

#[js_export]
#[derive(Clone)]
enum FormDataEntryValue {
    File(File),
    Blob(Blob),
    String(String),
}

#[js_export]
pub struct FormData {
    // Reasons for using Vec instead of HashMap:
    // 1. FormData specification requires maintaining insertion order, which Vec naturally supports
    // 2. FormData is mainly used for form data, where the data volume is usually small, so O(n) lookup performance impact is minimal
    // 3. The most common operation is iteration (entries/keys/values), for which Vec is more suitable
    // 4. Simplifies implementation by not needing to maintain multiple data structures to preserve order
    entries: Vec<(String, FormDataEntryValue, String)>,
}

// Iterator specifically for values() method
struct FormDataValuesIter {
    entries: Vec<(String, FormDataEntryValue, String)>,
    pos: usize,
}

impl Iterator for FormDataValuesIter {
    type Item = FormDataEntryValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.entries.len() {
            let value = self.entries[self.pos].1.clone();
            self.pos += 1;
            Some(value)
        } else {
            None
        }
    }
}

// Iterator specifically for keys() method
struct FormDataKeysIter {
    entries: Vec<(String, FormDataEntryValue, String)>,
    pos: usize,
}

impl Iterator for FormDataKeysIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.entries.len() {
            let key = self.entries[self.pos].0.clone();
            self.pos += 1;
            Some(key)
        } else {
            None
        }
    }
}

// Iterator for entries() method
struct FormDataEntriesIter {
    entries: Vec<(String, FormDataEntryValue, String)>,
    pos: usize,
    ctx: JSContext,
}

impl Iterator for FormDataEntriesIter {
    type Item = JSArray;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.entries.len() {
            let (key, value, _) = self.entries[self.pos].clone();
            self.pos += 1;
            let array = JSArray::new(&self.ctx).unwrap();
            array.push(key).unwrap();
            array.push(value).unwrap();
            Some(array)
        } else {
            None
        }
    }
}

#[js_class]
impl FormData {
    #[js_method(constructor)]
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// appends a new value onto an existing key inside a FormData object, or adds
    /// the key if it does not already exist.
    #[js_method]
    fn append(&mut self, name: String, value: FormDataEntryValue, filename: Optional<String>) {
        let filename = match &value {
            FormDataEntryValue::File(file) => filename.0.unwrap_or_else(|| file.name()),
            FormDataEntryValue::Blob(_) => filename.0.unwrap_or_else(|| "blob".to_string()),
            FormDataEntryValue::String(_) => String::new(),
        };
        self.entries.push((name, value, filename));
    }

    /// deletes a key and its value(s) from a FormData object
    #[js_method]
    fn delete(&mut self, name: String) {
        self.entries.retain(|(key, _, _)| key != &name);
    }

    /// returns the first value associated with a given key from within a FormData object
    #[js_method]
    fn get(&self, name: String) -> Option<FormDataEntryValue> {
        self.entries
            .iter()
            .find(|(key, _, _)| key == &name)
            .map(|(_, value, _)| value.clone())
    }

    /// returns all the values associated with a given key from within a FormData object.
    #[js_method(rename = "getAll")]
    fn get_all(&self, name: String) -> Vec<FormDataEntryValue> {
        self.entries
            .iter()
            .filter(|(key, _, _)| key == &name)
            .map(|(_, value, _)| value.clone())
            .collect()
    }

    /// returns whether a FormData object contains a certain key.
    #[js_method]
    fn has(&self, name: String) -> bool {
        self.entries.iter().any(|(key, _, _)| key == &name)
    }

    /// sets a new value for an existing key inside a FormData object, or adds the
    /// key/value if it does not already exist.
    #[js_method]
    fn set(&mut self, name: String, value: FormDataEntryValue, filename: Optional<String>) {
        self.delete(name.clone());
        self.append(name, value, filename);
    }

    /// returns an iterator which iterates through all values contained in the FormData
    #[js_method]
    fn values(&self, ctx: JSContext) -> JSResult<JSObject> {
        FormDataValuesIter {
            entries: self.entries.clone(),
            pos: 0,
        }
        .into_js_iter(&ctx)
    }

    /// returns an iterator which iterates through all key/value pairs contained
    /// in the FormData. The key of each pair is a string, and the value is either
    /// a string or a Blob
    #[js_method]
    fn entries(&self, ctx: JSContext) -> JSResult<JSObject> {
        FormDataEntriesIter {
            entries: self.entries.clone(),
            pos: 0,
            ctx: ctx.clone(),
        }
        .into_js_iter(&ctx)
    }

    /// returns an iterator which iterates through all keys contained in the FormData
    #[js_method]
    fn keys(&self, ctx: JSContext) -> JSResult<JSObject> {
        FormDataKeysIter {
            entries: self.entries.clone(),
            pos: 0,
        }
        .into_js_iter(&ctx)
    }
}

impl FormData {
    // Add new methods for serialization
    pub(crate) async fn serialize(&self, ctx: JSContext) -> JSResult<(Vec<u8>, String)> {
        let boundary = uuid::Uuid::new_v4().to_string();
        let mut body = Vec::new();

        for (name, value, filename) in &self.entries {
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());

            match value {
                FormDataEntryValue::File(file) => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                            name, filename
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(
                        format!("Content-Type: {}\r\n\r\n", file.mime_type()).as_bytes(),
                    );
                    if let Ok(bytes) = file.bytes(ctx.clone()).await {
                        if let Some(bytes_vec) = bytes.as_bytes() {
                            body.extend_from_slice(bytes_vec);
                        }
                    }
                }
                FormDataEntryValue::Blob(blob) => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                            name, filename
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(
                        format!("Content-Type: {}\r\n\r\n", blob.mime_type()).as_bytes(),
                    );
                    if let Ok(bytes) = blob.bytes(ctx.clone()).await {
                        if let Some(bytes_vec) = bytes.as_bytes() {
                            body.extend_from_slice(bytes_vec);
                        }
                    }
                }
                FormDataEntryValue::String(value) => {
                    body.extend_from_slice(
                        format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name)
                            .as_bytes(),
                    );
                    body.extend_from_slice(value.as_bytes());
                }
            }
            body.extend_from_slice(b"\r\n");
        }

        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
        Ok((body, boundary))
    }

    pub(crate) fn content_type(boundary: &str) -> String {
        format!("multipart/form-data; boundary={}", boundary)
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<FormData>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_formdata() {
        async_run!(|ctx: JSContext| async move {
            assert::init(&ctx)?;
            console::init(&ctx)?;
            encoding::init(&ctx)?;
            buffer::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "formdata.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
