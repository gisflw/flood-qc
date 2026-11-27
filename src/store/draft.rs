use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

// TOML has no null. A reserved tagged value preserves explicit absence inside
// recipes and other dynamically typed records. Ordinary objects with this key
// are escaped, so application data can never collide with the encoding.
fn encode(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Null => serde_json::json!({"__mgb_value": "null"}),
        Value::Array(values) => Value::Array(values.into_iter().map(encode).collect()),
        Value::Object(values) => {
            let escaped = values.contains_key("__mgb_value");
            let values: serde_json::Map<String, Value> =
                values.into_iter().map(|(k, v)| (k, encode(v))).collect();
            if escaped {
                serde_json::json!({"__mgb_value": "object", "entries": values})
            } else {
                Value::Object(values)
            }
        }
        other => other,
    }
}
fn decode(value: serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    match value {
        Value::Array(values) => Ok(Value::Array(
            values.into_iter().map(decode).collect::<Result<_, _>>()?,
        )),
        Value::Object(mut values) => match values.remove("__mgb_value") {
            Some(Value::String(tag)) if tag == "null" && values.is_empty() => Ok(Value::Null),
            Some(Value::String(tag)) if tag == "object" && values.len() == 1 => {
                let Value::Object(entries) =
                    values.remove("entries").ok_or("invalid escaped record")?
                else {
                    return Err("invalid escaped record".into());
                };
                Ok(Value::Object(
                    entries
                        .into_iter()
                        .map(|(k, v)| Ok((k, decode(v)?)))
                        .collect::<Result<_, String>>()?,
                ))
            }
            Some(_) => Err("invalid reserved TOML value".into()),
            None => Ok(Value::Object(
                values
                    .into_iter()
                    .map(|(k, v)| Ok((k, decode(v)?)))
                    .collect::<Result<_, String>>()?,
            )),
        },
        other => Ok(other),
    }
}
pub(crate) fn record_bytes(path: &Path, value: &impl Serialize) -> Result<Vec<u8>, String> {
    if path.extension().is_some_and(|e| e == "toml") {
        let data = encode(serde_json::to_value(value).map_err(|e| e.to_string())?);
        let data = serde_json::json!({"record": data});
        toml::to_string_pretty(&data)
            .map(String::into_bytes)
            .map_err(|e| e.to_string())
    } else {
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
    }
}
pub fn read_draft<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if path.extension().is_some_and(|e| e == "toml") {
        let value: toml::Value =
            toml::from_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
        let value = decode(value.get("record").ok_or("missing TOML record")?.clone())?;
        serde_json::from_value(value).map_err(|e| e.to_string())
    } else {
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}
pub fn write_draft<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    write_atomic(path, &record_bytes(path, value)?)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, true)
}

pub fn write_new_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, false)
}

fn write_file(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or("destination needs a filename")?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if replace {
            fs::rename(&temporary, path).map_err(|e| e.to_string())
        } else {
            fs::hard_link(&temporary, path).map_err(|e| e.to_string())?;
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
            super::sync_directory(parent)
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
