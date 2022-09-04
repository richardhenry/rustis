use crate::{
    resp::{Array, BulkString, Value},
    Error, Result,
};
use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

pub(crate) struct ValueDecoder;

impl Decoder for ValueDecoder {
    type Item = Value;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Value>> {
        Ok(decode(src, 0)?.map(|(item, pos)| {
            src.advance(pos);
            item
        }))
    }
}

fn decode(buf: &mut BytesMut, idx: usize) -> Result<Option<(Value, usize)>> {
    if buf.len() <= idx {
        return Ok(None);
    }

    let first_byte = buf[idx];
    let idx = idx + 1;

    match first_byte {
        b'$' => Ok(decode_bulk_string(buf, idx)?.map(|(bs, pos)| (Value::BulkString(bs), pos))),
        b'*' => Ok(decode_array(buf, idx)?.map(|(v, pos)| (Value::Array(Array::Vec(v)), pos))),
        b':' => Ok(decode_integer(buf, idx)?.map(|(i, pos)| (Value::Integer(i), pos))),
        b'+' => Ok(decode_string(buf, idx)?.map(|(s, pos)| (Value::SimpleString(s), pos))),
        b'-' => Ok(decode_string(buf, idx)?.map(|(s, pos)| (Value::Error(s), pos))),
        _ => Err(Error::Parse(format!(
            "Unknown data type '{}' (0x{:02x})",
            first_byte as char, first_byte
        ))),
    }
}

fn decode_bulk_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(BulkString, usize)>> {
    match decode_integer(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((BulkString::Nil, pos))),
        Some((len, pos)) => {
            if buf.len() - pos < len as usize + 2 {
                Ok(None) // EOF
            } else {
                if buf[pos + len as usize] != b'\r' || buf[pos + len as usize + 1] != b'\n' {
                    Err(Error::Parse(format!(
                        "Expected \\r\\n after bulk string. Got '{}''{}'",
                        buf[pos + len as usize] as char,
                        buf[pos + len as usize + 1] as char
                    )))
                } else {
                    Ok(Some((
                        BulkString::Binary(buf[pos..(pos + len as usize)].to_vec()),
                        pos + len as usize + 2,
                    )))
                }
            }
        }
    }
}

fn decode_array(buf: &mut BytesMut, idx: usize) -> Result<Option<(Vec<Value>, usize)>> {
    match decode_integer(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((Vec::new(), pos))),
        Some((len, pos)) => {
            let mut values = Vec::with_capacity(len as usize);
            let mut pos = pos;
            for _ in 0..len {
                match decode(buf, pos)? {
                    None => return Ok(None),
                    Some((value, new_pos)) => {
                        values.push(value);
                        pos = new_pos;
                    }
                }
            }
            Ok(Some((values, pos)))
        }
    }
}

fn decode_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(String, usize)>> {
    let len = buf.len();
    let mut pos = idx;
    let mut cr = false;

    while pos < len {
        let byte = buf[pos];

        match (cr, byte) {
            (false, b'\r') => cr = true,
            (true, b'\n') => {
                return Ok(Some((
                    String::from_utf8_lossy(&buf[idx..pos - 1]).into_owned(),
                    pos + 1,
                )))
            }
            (false, _) => (),
            _ => return Err(Error::Parse(format!("Unexpected byte {}", byte))),
        }

        pos = pos + 1;
    }

    Ok(None)
}

fn decode_integer(buf: &mut BytesMut, idx: usize) -> Result<Option<(i64, usize)>> {
    let len = buf.len();
    let mut is_negative = false;
    let mut i = 0i64;
    let mut pos = idx;
    let mut cr = false;

    while pos < len {
        let byte = buf[pos];

        match (cr, is_negative, byte) {
            (false, false, b'-') => is_negative = true,
            (false, false, b'0'..=b'9') => i = i * 10 + (byte - b'0') as i64,
            (false, true, b'0'..=b'9') => i = i * 10 - (byte - b'0') as i64,
            (false, _, b'\r') => cr = true,
            (true, _, b'\n') => return Ok(Some((i, pos + 1))),
            _ => return Err(Error::Parse(format!("Unexpected byte {}", byte))),
        }

        pos = pos + 1;
    }

    Ok(None)
}