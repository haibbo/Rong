use super::*;

/// Set a key-value pair in storage
pub async fn storage_set(key: String, value: JSValue) -> JSResult<()> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    // Validate key size
    if key.len() > DEFAULT_MAX_KEY_SIZE {
        return Err(RongJSError::TypeError(format!(
            "Key size exceeds maximum limit of {} bytes",
            DEFAULT_MAX_KEY_SIZE
        )));
    }

    // Convert value to JSON string to preserve type information
    let value_str = if value.is_string() {
        // For strings, store as JSON string to preserve type
        let s: String = value
            .clone()
            .try_into()
            .map_err(|_| RongJSError::TypeError("Failed to convert string value".to_string()))?;
        serde_json::to_string(&s)
            .map_err(|e| RongJSError::TypeError(format!("Failed to serialize string: {}", e)))?
    } else if value.is_number() {
        // First get as f64 to avoid truncation issues
        let f: f64 = value
            .clone()
            .try_into()
            .map_err(|_| RongJSError::TypeError("Failed to convert number value".to_string()))?;

        // Check if it's actually an integer (no fractional part)
        if f.fract() == 0.0 {
            // It's an integer, try to fit in appropriate integer type
            if f >= i32::MIN as f64 && f <= i32::MAX as f64 {
                // Fits in i32
                serde_json::to_string(&(f as i32)).map_err(|e| {
                    RongJSError::TypeError(format!("Failed to serialize i32: {}", e))
                })?
            } else if f >= 0.0 && f <= u32::MAX as f64 {
                // Fits in u32
                serde_json::to_string(&(f as u32)).map_err(|e| {
                    RongJSError::TypeError(format!("Failed to serialize u32: {}", e))
                })?
            } else {
                // Large integer, store as f64
                serde_json::to_string(&f).map_err(|e| {
                    RongJSError::TypeError(format!(
                        "Failed to serialize large integer as f64: {}",
                        e
                    ))
                })?
            }
        } else {
            // It's a floating point number
            serde_json::to_string(&f)
                .map_err(|e| RongJSError::TypeError(format!("Failed to serialize f64: {}", e)))?
        }
    } else if value.is_bigint() {
        // Handle BigInt values (i64/u64)
        if let Ok(i) = value.clone().try_into::<i64>() {
            serde_json::to_string(&i).map_err(|e| {
                RongJSError::TypeError(format!("Failed to serialize bigint i64: {}", e))
            })?
        } else if let Ok(u) = value.clone().try_into::<u64>() {
            serde_json::to_string(&u).map_err(|e| {
                RongJSError::TypeError(format!("Failed to serialize bigint u64: {}", e))
            })?
        } else {
            return Err(RongJSError::TypeError("Invalid bigint value".to_string()));
        }
    } else if value.is_boolean() {
        let b: bool = value
            .clone()
            .try_into()
            .map_err(|_| RongJSError::TypeError("Failed to convert boolean value".to_string()))?;
        serde_json::to_string(&b)
            .map_err(|e| RongJSError::TypeError(format!("Failed to serialize boolean: {}", e)))?
    } else if value.is_null() {
        "null".to_string()
    } else if value.is_undefined() {
        return Err(RongJSError::TypeError(
            "Cannot store undefined values".to_string(),
        ));
    } else if let Ok(date) = value.clone().try_into::<JSDate>() {
        // Handle Date objects by storing timestamp with type marker
        let timestamp = date
            .get_time()
            .map_err(|e| RongJSError::TypeError(format!("Failed to get Date timestamp: {}", e)))?;
        serde_json::to_string(&serde_json::json!({
            "__type": "Date",
            "timestamp": timestamp
        }))
        .map_err(|e| RongJSError::TypeError(format!("Failed to serialize Date: {}", e)))?
    } else if let Ok(obj) = value.clone().try_into::<JSObject>() {
        // Handle objects by converting to JSON string
        obj.json_stringify()
            .map_err(|e| RongJSError::TypeError(format!("Failed to stringify object: {}", e)))?
    } else if let Ok(s) = value.clone().try_into::<String>() {
        // Fallback: convert to string
        serde_json::to_string(&s).map_err(|e| {
            RongJSError::TypeError(format!("Failed to serialize fallback string: {}", e))
        })?
    } else {
        return Err(RongJSError::TypeError(
            "Value cannot be converted to a storable type".to_string(),
        ));
    };

    // Validate value size
    if value_str.len() > DEFAULT_MAX_VALUE_SIZE {
        return Err(RongJSError::TypeError(format!(
            "Value size exceeds maximum limit of {} bytes",
            DEFAULT_MAX_VALUE_SIZE
        )));
    }

    // Check total storage size before adding new data
    let read_txn = db
        .begin_read()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin read transaction: {}", e)))?;

    let table = read_txn
        .open_table(STORAGE_TABLE)
        .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

    let mut current_size = 0;
    let mut existing_key_size = 0;

    // Calculate current storage size and check if key already exists
    let iter = table
        .iter()
        .map_err(|e| RongJSError::TypeError(format!("Failed to create iterator: {}", e)))?;

    for item in iter {
        let (existing_key, existing_value) =
            item.map_err(|e| RongJSError::TypeError(format!("Failed to read item: {}", e)))?;

        let key_size = existing_key.value().len();
        let value_size = existing_value.value().len();

        if existing_key.value().as_bytes() == key.as_bytes() {
            existing_key_size = key_size + value_size;
        }
        current_size += key_size + value_size;
    }

    drop(table);
    drop(read_txn);

    // Calculate new size after this operation
    let new_entry_size = key.len() + value_str.len();
    let new_total_size = current_size - existing_key_size + new_entry_size;

    if new_total_size > DEFAULT_MAX_USER_DATA_SIZE {
        return Err(RongJSError::TypeError(format!(
            "Storage size would exceed maximum limit of {} bytes (current: {}, new entry: {})",
            DEFAULT_MAX_USER_DATA_SIZE,
            current_size - existing_key_size,
            new_entry_size
        )));
    }

    // Store in database
    let write_txn = db
        .begin_write()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin write transaction: {}", e)))?;

    {
        let mut table = write_txn
            .open_table(STORAGE_TABLE)
            .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

        table
            .insert(key.as_str(), value_str.as_bytes())
            .map_err(|e| RongJSError::TypeError(format!("Failed to insert value: {}", e)))?;
    }

    write_txn
        .commit()
        .map_err(|e| RongJSError::TypeError(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}

/// Get a value from storage
pub async fn storage_get(ctx: JSContext, key: String) -> JSResult<JSValue> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    let read_txn = db
        .begin_read()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin read transaction: {}", e)))?;

    let table = read_txn
        .open_table(STORAGE_TABLE)
        .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

    match table.get(key.as_str()) {
        Ok(Some(value)) => {
            let value_str = String::from_utf8(value.value().to_vec()).map_err(|e| {
                RongJSError::TypeError(format!("Failed to decode value as UTF-8: {}", e))
            })?;

            // Parse JSON back to appropriate JavaScript type
            match serde_json::from_str::<serde_json::Value>(&value_str) {
                Ok(json_value) => {
                    match json_value {
                        serde_json::Value::String(s) => Ok(JSValue::from(&ctx, s)),
                        serde_json::Value::Number(n) => {
                            // Let JSValue::from handle the intelligent number conversion
                            if let Some(i) = n.as_i64() {
                                Ok(JSValue::from(&ctx, i))
                            } else if let Some(u) = n.as_u64() {
                                Ok(JSValue::from(&ctx, u))
                            } else if let Some(f) = n.as_f64() {
                                Ok(JSValue::from(&ctx, f))
                            } else {
                                Ok(JSValue::from(&ctx, value_str))
                            }
                        }
                        serde_json::Value::Bool(b) => Ok(JSValue::from(&ctx, b)),
                        serde_json::Value::Null => Ok(JSValue::null(&ctx)),
                        serde_json::Value::Object(ref obj) => {
                            // Check if this is a Date object
                            if obj.get("__type")
                                == Some(&serde_json::Value::String("Date".to_string()))
                            {
                                if let Some(timestamp) =
                                    obj.get("timestamp").and_then(|v| v.as_f64())
                                {
                                    let date = JSDate::new(&ctx, timestamp);
                                    Ok(date.into_value())
                                } else {
                                    Err(RongJSError::TypeError(
                                        "Invalid Date object: missing timestamp".to_string(),
                                    ))
                                }
                            } else {
                                // Regular object, parse using JavaScript's JSON.parse
                                value_str.as_str().json_to_jsvalue(&ctx)
                            }
                        }
                        serde_json::Value::Array(_) => {
                            // For arrays, parse them back using JavaScript's JSON.parse
                            value_str.as_str().json_to_jsvalue(&ctx)
                        }
                    }
                }
                Err(_) => {
                    // If not valid JSON, return as string
                    Ok(JSValue::from(&ctx, value_str))
                }
            }
        }
        Ok(None) => Ok(JSValue::undefined(&ctx)),
        Err(e) => Err(RongJSError::TypeError(format!(
            "Failed to get value: {}",
            e
        ))),
    }
}

/// Delete a key from storage
pub async fn storage_delete(key: String) -> JSResult<()> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    let write_txn = db
        .begin_write()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin write transaction: {}", e)))?;

    {
        let mut table = write_txn
            .open_table(STORAGE_TABLE)
            .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

        table
            .remove(key.as_str())
            .map_err(|e| RongJSError::TypeError(format!("Failed to remove key: {}", e)))?;
    }

    write_txn
        .commit()
        .map_err(|e| RongJSError::TypeError(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}

/// Clear all data from storage
pub async fn storage_clear() -> JSResult<()> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    let write_txn = db
        .begin_write()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin write transaction: {}", e)))?;

    {
        let mut table = write_txn
            .open_table(STORAGE_TABLE)
            .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

        // Remove all entries
        let keys: Vec<String> = table
            .iter()
            .map_err(|e| RongJSError::TypeError(format!("Failed to iterate table: {}", e)))?
            .map(|item| {
                item.map(|(key, _)| key.value().to_string())
                    .map_err(|e| RongJSError::TypeError(format!("Failed to read key: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for key in keys {
            table
                .remove(key.as_str())
                .map_err(|e| RongJSError::TypeError(format!("Failed to remove key: {}", e)))?;
        }
    }

    write_txn
        .commit()
        .map_err(|e| RongJSError::TypeError(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}
