use sha1::{Digest, Sha1};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BValue {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<BValue>),
    Dict(BTreeMap<Vec<u8>, BValue>),
}

impl BValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            BValue::Bytes(b) => std::str::from_utf8(b).ok(),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        self.as_str().map(|s| s.to_string())
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            BValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[BValue]> {
        match self {
            BValue::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&BTreeMap<Vec<u8>, BValue>> {
        match self {
            BValue::Dict(d) => Some(d),
            _ => None,
        }
    }

    pub fn get_dict_entry(&self, key: &str) -> Option<&BValue> {
        self.as_dict().and_then(|d| d.get(key.as_bytes()))
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_into(&mut buf);
        buf
    }

    pub fn encode_into(&self, buf: &mut Vec<u8>) {
        match self {
            BValue::Int(i) => {
                buf.push(b'i');
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.push(b'e');
            }
            BValue::Bytes(b) => {
                buf.extend_from_slice(b.len().to_string().as_bytes());
                buf.push(b':');
                buf.extend_from_slice(b);
            }
            BValue::List(l) => {
                buf.push(b'l');
                for item in l {
                    item.encode_into(buf);
                }
                buf.push(b'e');
            }
            BValue::Dict(d) => {
                buf.push(b'd');
                for (k, v) in d {
                    buf.extend_from_slice(k.len().to_string().as_bytes());
                    buf.push(b':');
                    buf.extend_from_slice(k);
                    v.encode_into(buf);
                }
                buf.push(b'e');
            }
        }
    }

    pub fn sha1_digest(&self) -> [u8; 20] {
        let encoded = self.encode();
        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        hasher.finalize().into()
    }
}

pub fn decode(input: &[u8]) -> Result<BValue, String> {
    let mut idx = 0;
    let val = parse_value(input, &mut idx)?;
    Ok(val)
}

fn parse_value(input: &[u8], idx: &mut usize) -> Result<BValue, String> {
    if *idx >= input.len() {
        return Err("Unexpected end of bencoded input".to_string());
    }

    match input[*idx] {
        b'i' => parse_int(input, idx),
        b'l' => parse_list(input, idx),
        b'd' => parse_dict(input, idx),
        b'0'..=b'9' => parse_bytes(input, idx),
        other => Err(format!(
            "Invalid bencode prefix '{}' at offset {}",
            other as char, *idx
        )),
    }
}

fn parse_int(input: &[u8], idx: &mut usize) -> Result<BValue, String> {
    *idx += 1; // skip 'i'
    let start = *idx;
    while *idx < input.len() && input[*idx] != b'e' {
        *idx += 1;
    }
    if *idx >= input.len() {
        return Err("Unterminated integer".to_string());
    }
    let slice = &input[start..*idx];
    *idx += 1; // skip 'e'
    let s = std::str::from_utf8(slice).map_err(|e| e.to_string())?;
    let val = s.parse::<i64>().map_err(|e| e.to_string())?;
    Ok(BValue::Int(val))
}

fn parse_bytes(input: &[u8], idx: &mut usize) -> Result<BValue, String> {
    let start = *idx;
    while *idx < input.len() && input[*idx] != b':' {
        if !input[*idx].is_ascii_digit() {
            return Err(format!(
                "Invalid length character in byte string at {}",
                *idx
            ));
        }
        *idx += 1;
    }
    if *idx >= input.len() {
        return Err("Unterminated byte string length".to_string());
    }
    let len_str = std::str::from_utf8(&input[start..*idx]).map_err(|e| e.to_string())?;
    let len = len_str.parse::<usize>().map_err(|e| e.to_string())?;
    *idx += 1; // skip ':'
    if *idx + len > input.len() {
        return Err("Byte string length exceeds buffer".to_string());
    }
    let bytes = input[*idx..*idx + len].to_vec();
    *idx += len;
    Ok(BValue::Bytes(bytes))
}

fn parse_list(input: &[u8], idx: &mut usize) -> Result<BValue, String> {
    *idx += 1; // skip 'l'
    let mut list = Vec::new();
    while *idx < input.len() && input[*idx] != b'e' {
        list.push(parse_value(input, idx)?);
    }
    if *idx >= input.len() {
        return Err("Unterminated list".to_string());
    }
    *idx += 1; // skip 'e'
    Ok(BValue::List(list))
}

fn parse_dict(input: &[u8], idx: &mut usize) -> Result<BValue, String> {
    *idx += 1; // skip 'd'
    let mut dict = BTreeMap::new();
    while *idx < input.len() && input[*idx] != b'e' {
        let key_val = parse_bytes(input, idx)?;
        let key = match key_val {
            BValue::Bytes(b) => b,
            _ => return Err("Dict key must be byte string".to_string()),
        };
        let val = parse_value(input, idx)?;
        dict.insert(key, val);
    }
    if *idx >= input.len() {
        return Err("Unterminated dictionary".to_string());
    }
    *idx += 1; // skip 'e'
    Ok(BValue::Dict(dict))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bencode_roundtrip() {
        let input = b"d4:spaml1:a1:bee";
        let val = decode(input).unwrap();
        assert_eq!(val.encode(), input);
    }

    #[test]
    fn test_bencode_int() {
        let input = b"i42e";
        let val = decode(input).unwrap();
        assert_eq!(val, BValue::Int(42));
        assert_eq!(val.encode(), input);
    }
}
