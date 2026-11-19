use std::{ops::Range, str::FromStr};

pub(super) fn lines(bytes: &[u8]) -> impl Iterator<Item = (Range<usize>, &[u8])> {
    let mut start = 0;
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(move |line| {
            let offset = start;
            start += line.len();
            let body = line.strip_suffix(b"\n").unwrap_or(line);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            (offset..offset + body.len(), body)
        })
}

pub(super) fn parse<T: FromStr>(bytes: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(bytes.trim_ascii()).map_err(|_| "invalid numeric value")?;
    text.parse()
        .map_err(|_| format!("invalid numeric value {text:?}"))
}

pub(super) fn number(bytes: &[u8]) -> Result<f64, String> {
    let value: f64 = if let Some(index) = bytes.iter().position(|b| matches!(b, b'D' | b'd')) {
        let mut text = bytes.to_vec();
        text[index] = b'E';
        parse(&text)?
    } else {
        parse(bytes)?
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("expected a finite value".into())
    }
}

pub(super) fn formatted(text: String, width: usize) -> Result<Vec<u8>, String> {
    if text.len() > width {
        return Err(format!("value {text} does not fit the engine field"));
    }
    Ok(format!("{text:>width$}").into_bytes())
}
