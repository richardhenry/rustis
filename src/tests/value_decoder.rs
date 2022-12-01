use bytes::BytesMut;
use tokio_util::codec::Decoder;

use crate::{
    resp::{Value, ValueDecoder},
    tests::log_try_init,
    RedisError, RedisErrorKind, Result,
};

fn decode_value(str: &str) -> Result<Option<Value>> {
    let mut buf = BytesMut::from(str);
    let mut value_decoder = ValueDecoder;
    value_decoder.decode(&mut buf)
}

#[test]
fn simple_string() -> Result<()> {
    log_try_init();

    let result = decode_value("+OK\r\n")?; // "OK"
    assert_eq!(Some(Value::SimpleString("OK".to_owned())), result);

    let result = decode_value("+OK\r")?;
    assert_eq!(None, result);

    let result = decode_value("+OK")?;
    assert_eq!(None, result);

    let result = decode_value("+")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let result = decode_value(":12\r\n")?; // 12
    assert_eq!(Some(Value::Integer(12)), result);

    let result = decode_value(":12\r")?;
    assert_eq!(None, result);

    let result = decode_value(":12")?;
    assert_eq!(None, result);

    let result = decode_value(":")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    log_try_init();

    let result = decode_value(",12.12\r\n")?; // 12.12
    assert_eq!(Some(Value::Double(12.12)), result);

    let result = decode_value(",12.12\r")?;
    assert_eq!(None, result);

    let result = decode_value(",12.12")?;
    assert_eq!(None, result);

    let result = decode_value(",")?;
    assert_eq!(None, result);

    let result = decode_value(",12a\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn boolean() -> Result<()> {
    log_try_init();

    let result = decode_value("#f\r\n")?; // false
    assert_eq!(Some(Value::Integer(0)), result);

    let result = decode_value("#t\r\n")?; // true
    assert_eq!(Some(Value::Integer(1)), result);

    let result = decode_value("#f\r")?;
    assert_eq!(None, result);

    let result = decode_value("#f")?;
    assert_eq!(None, result);

    let result = decode_value("#")?;
    assert_eq!(None, result);

    let result = decode_value("#wrong\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn null() -> Result<()> {
    log_try_init();

    let result = decode_value("_\r\n")?; // null
    assert_eq!(Some(Value::BulkString(None)), result);

    let result = decode_value("_\r")?;
    assert_eq!(None, result);

    let result = decode_value("_")?;
    assert_eq!(None, result);

    let result = decode_value("_wrong\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn bulk_string() -> Result<()> {
    log_try_init();

    let result = decode_value("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!(Some(Value::BulkString(Some(b"hello".to_vec()))), result);

    let result = decode_value("$7\r\nhel\r\nlo\r\n")?; // b"hel\r\nlo"
    assert_eq!(Some(Value::BulkString(Some(b"hel\r\nlo".to_vec()))), result);

    let result = decode_value("$5\r\nhello\r")?;
    assert_eq!(None, result);

    let result = decode_value("$5\r\nhello")?;
    assert_eq!(None, result);

    let result = decode_value("$5\r")?;
    assert_eq!(None, result);

    let result = decode_value("$5")?;
    assert_eq!(None, result);

    let result = decode_value("$")?;
    assert_eq!(None, result);

    let result = decode_value("$6\r\nhello\r\n");
    log::debug!("result: {result:?}");
    assert!(result.is_err());

    let result = decode_value("$-1\r\n")?; // b""
    assert_eq!(Some(Value::BulkString(None)), result);

    let result = decode_value("$-1\r")?;
    assert_eq!(None, result);

    let result = decode_value("$-1")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn array() -> Result<()> {
    log_try_init();

    let result = decode_value("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Some(Value::Array(Some(vec![
            Value::Integer(12),
            Value::Integer(13)
        ]))),
        result
    );

    let result = decode_value("*2\r\n:12\r\n:13\r")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12\r\n:13")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12\r\n")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r")?;
    assert_eq!(None, result);

    let result = decode_value("*2")?;
    assert_eq!(None, result);

    let result = decode_value("*")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Some(Value::Array(Some(vec![
            Value::BulkString(Some(b"hello".to_vec())),
            Value::BulkString(Some(b"world".to_vec()))
        ]))),
        result
    );

    let result = decode_value("*-1\r\n")?; // []
    assert_eq!(Some(Value::Array(None)), result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Some(Value::Array(Some(vec![
            Value::BulkString(Some(b"id".to_vec())),
            Value::Integer(12),
            Value::BulkString(Some(b"name".to_vec())),
            Value::BulkString(Some(b"Mike".to_vec()))
        ]))),
        result
    );

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r")?;
    assert_eq!(None, result);

    let result = decode_value("%2")?;
    assert_eq!(None, result);

    let result = decode_value("%")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn set() -> Result<()> {
    log_try_init();

    let result = decode_value("~2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Some(Value::Array(Some(vec![
            Value::Integer(12),
            Value::Integer(13)
        ]))),
        result
    );

    let result = decode_value("~2\r\n:12\r\n:13\r")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12\r\n:13")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12\r\n")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r")?;
    assert_eq!(None, result);

    let result = decode_value("~2")?;
    assert_eq!(None, result);

    let result = decode_value("~")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Some(Value::Array(Some(vec![
            Value::BulkString(Some(b"hello".to_vec())),
            Value::BulkString(Some(b"world".to_vec()))
        ]))),
        result
    );

    Ok(())
}

#[test]
fn push() -> Result<()> {
    log_try_init();

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r\n")?; // [b"message, b"channel", b"payload"]
    assert_eq!(
        Some(Value::Push(Some(vec![
            Value::BulkString(Some(b"message".to_vec())),
            Value::BulkString(Some(b"channel".to_vec())),
            Value::BulkString(Some(b"payload".to_vec()))
        ]))),
        result
    );

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r")?;
    assert_eq!(None, result);

    let result = decode_value(">3")?;
    assert_eq!(None, result);

    let result = decode_value(">")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    log_try_init();

    let result = decode_value("-ERR error\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Ok(Some(Value::Error(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })))
    ));

    let result = decode_value("-ERR error\r")?;
    assert_eq!(None, result);

    let result = decode_value("-ERR error")?;
    assert_eq!(None, result);

    let result = decode_value("-")?;
    assert_eq!(None, result);

    Ok(())
}
