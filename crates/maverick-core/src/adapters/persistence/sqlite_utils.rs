use crate::db::{Row, Value};
use crate::error::{AppError, Result};

pub fn blob_literal(bytes: &[u8]) -> String {
    format!("X'{}'", encode_hex(bytes))
}

pub fn optional_blob_literal(bytes: Option<&[u8]>) -> String {
    bytes
        .map(blob_literal)
        .unwrap_or_else(|| "NULL".to_string())
}

pub fn text_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn optional_text_literal(value: Option<&str>) -> String {
    value
        .map(text_literal)
        .unwrap_or_else(|| "NULL".to_string())
}

pub fn optional_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NULL".to_string())
}

pub fn optional_real(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NULL".to_string())
}

pub fn required_blob<const N: usize>(
    row: &Row,
    index: usize,
    field: &'static str,
) -> Result<[u8; N]> {
    match row.values.get(index) {
        Some(Value::Blob(bytes)) if bytes.len() == N => {
            let mut out = [0u8; N];
            out.copy_from_slice(bytes);
            Ok(out)
        }
        _ => Err(AppError::Database(format!(
            "invalid blob for field {field}"
        ))),
    }
}

pub fn required_text(row: &Row, index: usize, field: &'static str) -> Result<String> {
    match row.values.get(index) {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(AppError::Database(format!(
            "invalid text for field {field}"
        ))),
    }
}

pub fn required_i64(row: &Row, index: usize, field: &'static str) -> Result<i64> {
    match row.values.get(index) {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(AppError::Database(format!(
            "invalid integer for field {field}"
        ))),
    }
}

pub fn optional_text(row: &Row, index: usize) -> Option<String> {
    match row.values.get(index) {
        Some(Value::Text(value)) => Some(value.clone()),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{:02X}", byte);
    }

    output
}
