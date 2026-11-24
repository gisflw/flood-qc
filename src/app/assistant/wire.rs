use super::invalid_data;
use serde_json::{Value, json};
use std::{io, time::Duration};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

pub(super) const MAX_LINE: usize = 262_144;
pub(super) const IO_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct Reader {
    line: Vec<u8>,
}

impl Reader {
    /// Retains partial lines when another branch of the conversation makes progress.
    pub(super) async fn next(
        &mut self,
        source: &mut (impl AsyncBufRead + Unpin),
    ) -> io::Result<Option<Value>> {
        loop {
            let bytes = source.fill_buf().await?;
            if bytes.is_empty() {
                return if self.line.is_empty() {
                    Ok(None)
                } else {
                    Err(invalid_data("truncated JSON-RPC line"))
                };
            }
            let newline = bytes.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(bytes.len(), |index| index + 1);
            if self.line.len() + consumed > MAX_LINE {
                return Err(invalid_data("JSON-RPC line exceeds the size limit"));
            }
            self.line.extend_from_slice(&bytes[..consumed]);
            source.consume(consumed);
            if newline.is_some() {
                let parsed = serde_json::from_slice::<Value>(&self.line)
                    .map_err(|_| invalid_data("malformed JSON-RPC line"))?;
                self.line.clear();
                if parsed.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
                    return Err(invalid_data("invalid JSON-RPC envelope"));
                }
                return Ok(Some(parsed));
            }
        }
    }
}

pub(super) async fn send(
    destination: &mut (impl AsyncWrite + Unpin),
    message: &Value,
) -> io::Result<()> {
    let mut payload = serde_json::to_vec(message).map_err(io::Error::other)?;
    payload.push(b'\n');
    if payload.len() > MAX_LINE {
        return Err(invalid_data("JSON-RPC line exceeds the size limit"));
    }
    tokio::time::timeout(IO_TIMEOUT, async {
        destination.write_all(&payload).await?;
        destination.flush().await
    })
    .await?
}

pub(super) fn response(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":result})
}

pub(super) fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

pub(super) fn request(id: Option<u64>, method: &str, parameters: Value) -> Value {
    let mut message = json!({"jsonrpc":"2.0","method":method,"params":parameters});
    if let Some(id) = id {
        message["id"] = json!(id);
    }
    message
}
