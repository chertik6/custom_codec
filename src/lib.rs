use std::io::{Cursor, Read};

/// Типы поддерживаемых значений
#[derive(Debug, PartialEq)]
pub enum Value {
    Int32(i32),
    Float32(f32),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Message(Vec<Field>), // вложенное сообщение
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub key: String,
    pub value: Value,
}

/// Кодирование одного поля
pub fn encode_field(field: &Field) -> Vec<u8> {
    let mut out = Vec::new();

    // 1 байт type_code
    let type_code: u8 = match field.value {
        Value::Int32(_) => 1,
        Value::Float32(_) => 2,
        Value::Bool(_) => 3,
        Value::String(_) => 4,
        Value::Bytes(_) => 5,
        Value::Message(_) => 6,
    };
    out.push(type_code);

    // длина ключа (4 байта big-endian)
    let key_bytes = field.key.as_bytes();
    out.extend_from_slice(&(key_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(key_bytes);

    // значение
    match &field.value {
        Value::Int32(i) => {
            out.extend_from_slice(&(4u32).to_be_bytes());
            out.extend_from_slice(&i.to_be_bytes());
        }
        Value::Float32(f) => {
            out.extend_from_slice(&(4u32).to_be_bytes());
            out.extend_from_slice(&f.to_be_bytes());
        }
        Value::Bool(b) => {
            out.extend_from_slice(&(1u32).to_be_bytes());
            out.push(*b as u8);
        }
        Value::String(s) => {
            let bytes = s.as_bytes();
            out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        Value::Bytes(bts) => {
            out.extend_from_slice(&(bts.len() as u32).to_be_bytes());
            out.extend_from_slice(bts);
        }
        Value::Message(fields) => {
            let mut inner = Vec::new();
            for f in fields {
                inner.extend_from_slice(&encode_field(f));
            }
            out.extend_from_slice(&(inner.len() as u32).to_be_bytes());
            out.extend_from_slice(&inner);
        }
    }

    out
}

/// Декодирование одного поля
pub fn decode_field(data: &[u8]) -> Option<Field> {
    let mut cur = Cursor::new(data);

    let mut type_code = [0u8; 1];
    cur.read_exact(&mut type_code).ok()?;

    let mut len_buf = [0u8; 4];

    // длина ключа
    cur.read_exact(&mut len_buf).ok()?;
    let key_len = u32::from_be_bytes(len_buf) as usize;

    let mut key_bytes = vec![0u8; key_len];
    cur.read_exact(&mut key_bytes).ok()?;
    let key = String::from_utf8(key_bytes).ok()?;

    // длина значения
    cur.read_exact(&mut len_buf).ok()?;
    let val_len = u32::from_be_bytes(len_buf) as usize;

    let mut val_bytes = vec![0u8; val_len];
    cur.read_exact(&mut val_bytes).ok()?;

    let value = match type_code[0] {
        1 => {
            let mut arr = [0u8;4];
            arr.copy_from_slice(&val_bytes);
            Value::Int32(i32::from_be_bytes(arr))
        }
        2 => {
            let mut arr = [0u8;4];
            arr.copy_from_slice(&val_bytes);
            Value::Float32(f32::from_be_bytes(arr))
        }
        3 => Value::Bool(val_bytes[0] != 0),
        4 => Value::String(String::from_utf8(val_bytes).ok()?),
        5 => Value::Bytes(val_bytes),
        6 => {
            let mut inner = Vec::new();
            let mut slice = &val_bytes[..];
            while !slice.is_empty() {
                if let Some(f) = decode_field(slice) {
                    let encoded = encode_field(&f);
                    let take = encoded.len();
                    inner.push(f);
                    slice = &slice[take..];
                } else {
                    break;
                }
            }
            Value::Message(inner)
        }
        _ => return None,
    };

    Some(Field { key, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_roundtrip() {
        let f = Field { key: "age".into(), value: Value::Int32(42) };
        let enc = encode_field(&f);
        let dec = decode_field(&enc).unwrap();
        assert_eq!(f, dec);
    }

    #[test]
    fn string_roundtrip() {
        let f = Field { key: "name".into(), value: Value::String("Rust".into()) };
        let enc = encode_field(&f);
        let dec = decode_field(&enc).unwrap();
        assert_eq!(f, dec);
    }
}
