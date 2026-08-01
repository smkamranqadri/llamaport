use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;

const MAGIC: [u8; 4] = *b"GGUF";
const MAX_KV_COUNT: u64 = 1_000_000;
const MAX_STRING_LEN: u64 = 64 * 1024 * 1024;
const MAX_INLINE_ARRAY: u64 = 1024;

#[derive(Debug)]
pub enum ParseError {
    Io(io::Error),
    NotGguf,
    UnsupportedVersion(u32),
    UnknownValueType(u32),
    NestedArray,
    Malformed(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "{e}"),
            ParseError::NotGguf => write!(f, "not a GGUF file (bad magic)"),
            ParseError::UnsupportedVersion(v) => write!(f, "unsupported GGUF version {v}"),
            ParseError::UnknownValueType(t) => write!(f, "unknown metadata value type {t}"),
            ParseError::NestedArray => write!(f, "nested arrays are not supported"),
            ParseError::Malformed(what) => write!(f, "malformed header: {what}"),
        }
    }
}

impl From<io::Error> for ParseError {
    fn from(e: io::Error) -> Self {
        ParseError::Io(e)
    }
}

type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone)]
pub enum Value {
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
    Ints(Vec<i64>),
    Skipped,
}

impl Value {
    fn as_u64(&self) -> Option<u64> {
        match self {
            Value::U64(v) => Some(*v),
            Value::I64(v) if *v >= 0 => Some(*v as u64),
            // Some architectures store per-layer values; the largest is what sizing must assume.
            Value::Ints(v) => v.iter().copied().filter(|n| *n >= 0).max().map(|n| n as u64),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

struct Reader<R: Read + Seek> {
    inner: R,
}

impl<R: Read + Seek> Reader<R> {
    fn u8(&mut self) -> Result<u8> {
        let mut b = [0u8; 1];
        self.inner.read_exact(&mut b)?;
        Ok(b[0])
    }

    fn u16(&mut self) -> Result<u16> {
        let mut b = [0u8; 2];
        self.inner.read_exact(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn u32(&mut self) -> Result<u32> {
        let mut b = [0u8; 4];
        self.inner.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn u64(&mut self) -> Result<u64> {
        let mut b = [0u8; 8];
        self.inner.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }

    fn string(&mut self) -> Result<String> {
        let len = self.u64()?;
        if len > MAX_STRING_LEN {
            return Err(ParseError::Malformed("implausible string length"));
        }
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

impl<R: Read + Seek> Reader<R> {
    /// Seeking invalidates the read buffer, so short skips are cheaper read than sought.
    fn skip(&mut self, n: u64) -> Result<()> {
        if n > 1 << 16 {
            self.inner.seek(SeekFrom::Current(n as i64))?;
        } else {
            io::copy(&mut self.inner.by_ref().take(n), &mut io::sink())?;
        }
        Ok(())
    }
}

fn fixed_size(value_type: u32) -> Option<u64> {
    match value_type {
        0 | 1 | 7 => Some(1),
        2 | 3 => Some(2),
        4 | 5 | 6 => Some(4),
        10 | 11 | 12 => Some(8),
        _ => None,
    }
}

fn is_integer(value_type: u32) -> bool {
    matches!(value_type, 0 | 1 | 2 | 3 | 4 | 5 | 10 | 11)
}

impl<R: Read + Seek> Reader<R> {
    fn value(&mut self, value_type: u32) -> Result<Value> {
        match value_type {
            0 => Ok(Value::U64(self.u8()? as u64)),
            1 => Ok(Value::I64(self.u8()? as i8 as i64)),
            2 => Ok(Value::U64(self.u16()? as u64)),
            3 => Ok(Value::I64(self.u16()? as i16 as i64)),
            4 => Ok(Value::U64(self.u32()? as u64)),
            5 => Ok(Value::I64(self.u32()? as i32 as i64)),
            6 => Ok(Value::F64(self.f32()? as f64)),
            7 => Ok(Value::Bool(self.u8()? != 0)),
            8 => Ok(Value::Str(self.string()?)),
            9 => self.array(),
            10 => Ok(Value::U64(self.u64()?)),
            11 => Ok(Value::I64(self.u64()? as i64)),
            12 => Ok(Value::F64(self.f64()?)),
            other => Err(ParseError::UnknownValueType(other)),
        }
    }

    fn integer(&mut self, value_type: u32) -> Result<i64> {
        match self.value(value_type)? {
            Value::U64(v) => Ok(v as i64),
            Value::I64(v) => Ok(v),
            _ => Err(ParseError::Malformed("expected integer array element")),
        }
    }

    fn array(&mut self) -> Result<Value> {
        let elem_type = self.u32()?;
        let len = self.u64()?;

        if elem_type == 9 {
            return Err(ParseError::NestedArray);
        }

        if is_integer(elem_type) && len <= MAX_INLINE_ARRAY {
            let mut out = Vec::with_capacity(len as usize);
            for _ in 0..len {
                out.push(self.integer(elem_type)?);
            }
            return Ok(Value::Ints(out));
        }

        match fixed_size(elem_type) {
            Some(size) => self.skip(len.saturating_mul(size))?,
            // Token lists run to hundreds of thousands of entries, so length-prefixed
            // strings have to be walked one at a time.
            None => {
                for _ in 0..len {
                    let n = self.u64()?;
                    if n > MAX_STRING_LEN {
                        return Err(ParseError::Malformed("implausible string length in array"));
                    }
                    self.skip(n)?;
                }
            }
        }

        Ok(Value::Skipped)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GgufMetadata {
    pub gguf_version: u32,
    pub tensor_count: u64,
    pub architecture: String,
    pub name: Option<String>,
    pub size_label: Option<String>,
    pub context_length: Option<u64>,
    pub block_count: Option<u64>,
    pub embedding_length: Option<u64>,
    pub head_count: Option<u64>,
    pub head_count_kv: Option<u64>,
    pub key_length: Option<u64>,
    pub value_length: Option<u64>,
    pub expert_count: Option<u64>,
    pub file_type: Option<u64>,
    pub has_chat_template: bool,
}

impl GgufMetadata {
    /// Per-head key dimension, needed for the KV cache size estimate.
    pub fn head_dim(&self) -> Option<u64> {
        if let Some(k) = self.key_length {
            return Some(k);
        }
        let embedding = self.embedding_length?;
        let heads = self.head_count?;
        if heads == 0 {
            return None;
        }
        Some(embedding / heads)
    }

    /// Latent-attention architectures (deepseek2 and friends) size V differently from K,
    /// so the cache estimate cannot assume one dimension for both.
    pub fn value_head_dim(&self) -> Option<u64> {
        self.value_length.or_else(|| self.head_dim())
    }

    pub fn is_moe(&self) -> bool {
        self.expert_count.unwrap_or(0) > 0
    }
}

pub fn read_metadata(path: &Path) -> Result<GgufMetadata> {
    let file = File::open(path)?;
    parse(BufReader::with_capacity(1 << 16, file))
}

pub fn parse<R: Read + Seek>(inner: R) -> Result<GgufMetadata> {
    let mut reader = Reader { inner };

    let mut magic = [0u8; 4];
    reader.inner.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(ParseError::NotGguf);
    }

    let gguf_version = reader.u32()?;
    if !(2..=3).contains(&gguf_version) {
        return Err(ParseError::UnsupportedVersion(gguf_version));
    }

    let tensor_count = reader.u64()?;
    let kv_count = reader.u64()?;
    if kv_count > MAX_KV_COUNT {
        return Err(ParseError::Malformed("implausible metadata count"));
    }

    let mut kv: HashMap<String, Value> = HashMap::with_capacity(kv_count as usize);
    let mut has_chat_template = false;

    for _ in 0..kv_count {
        let key = reader.string()?;
        let value_type = reader.u32()?;
        let value = reader.value(value_type)?;

        if key == "tokenizer.chat_template" {
            has_chat_template = true;
        }
        kv.insert(key, value);
    }

    let architecture = kv
        .get("general.architecture")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    let scoped = |suffix: &str| -> Option<u64> {
        kv.get(&format!("{architecture}.{suffix}"))
            .and_then(Value::as_u64)
    };

    Ok(GgufMetadata {
        gguf_version,
        tensor_count,
        name: kv
            .get("general.name")
            .and_then(Value::as_str)
            .map(str::to_string),
        size_label: kv
            .get("general.size_label")
            .and_then(Value::as_str)
            .map(str::to_string),
        context_length: scoped("context_length"),
        block_count: scoped("block_count"),
        embedding_length: scoped("embedding_length"),
        head_count: scoped("attention.head_count"),
        head_count_kv: scoped("attention.head_count_kv"),
        key_length: scoped("attention.key_length"),
        value_length: scoped("attention.value_length"),
        expert_count: scoped("expert_count"),
        file_type: kv.get("general.file_type").and_then(Value::as_u64),
        has_chat_template,
        architecture,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Default)]
    struct Builder {
        pairs: Vec<u8>,
        count: u64,
    }

    fn encode_string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u64).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    impl Builder {
        fn key(&mut self, key: &str, value_type: u32) {
            encode_string(&mut self.pairs, key);
            self.pairs.extend_from_slice(&value_type.to_le_bytes());
            self.count += 1;
        }

        fn string(mut self, key: &str, value: &str) -> Self {
            self.key(key, 8);
            encode_string(&mut self.pairs, value);
            self
        }

        fn u32(mut self, key: &str, value: u32) -> Self {
            self.key(key, 4);
            self.pairs.extend_from_slice(&value.to_le_bytes());
            self
        }

        fn string_array(mut self, key: &str, values: &[&str]) -> Self {
            self.key(key, 9);
            self.pairs.extend_from_slice(&8u32.to_le_bytes());
            self.pairs.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for v in values {
                encode_string(&mut self.pairs, v);
            }
            self
        }

        fn i32_array(mut self, key: &str, values: &[i32]) -> Self {
            self.key(key, 9);
            self.pairs.extend_from_slice(&5u32.to_le_bytes());
            self.pairs.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for v in values {
                self.pairs.extend_from_slice(&v.to_le_bytes());
            }
            self
        }

        fn finish(self) -> Cursor<Vec<u8>> {
            let mut out = Vec::new();
            out.extend_from_slice(&MAGIC);
            out.extend_from_slice(&3u32.to_le_bytes());
            out.extend_from_slice(&0u64.to_le_bytes());
            out.extend_from_slice(&self.count.to_le_bytes());
            out.extend_from_slice(&self.pairs);
            Cursor::new(out)
        }
    }

    fn minimal() -> Builder {
        Builder::default().string("general.architecture", "llama")
    }

    #[test]
    fn reads_scalar_metadata() {
        let md = parse(
            minimal()
                .string("general.name", "Test Model")
                .string("general.size_label", "7B")
                .u32("llama.context_length", 32768)
                .u32("llama.block_count", 32)
                .u32("llama.embedding_length", 4096)
                .u32("llama.attention.head_count", 32)
                .u32("llama.attention.head_count_kv", 8)
                .finish(),
        )
        .unwrap();

        assert_eq!(md.architecture, "llama");
        assert_eq!(md.name.as_deref(), Some("Test Model"));
        assert_eq!(md.size_label.as_deref(), Some("7B"));
        assert_eq!(md.context_length, Some(32768));
        assert_eq!(md.block_count, Some(32));
        assert_eq!(md.head_count_kv, Some(8));
        assert_eq!(md.head_dim(), Some(128));
        assert!(!md.has_chat_template);
        assert!(!md.is_moe());
    }

    #[test]
    fn walks_past_large_string_arrays() {
        let tokens: Vec<String> = (0..5000).map(|i| format!("token_{i}")).collect();
        let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();

        let md = parse(
            minimal()
                .string_array("tokenizer.ggml.tokens", &refs)
                .string("tokenizer.chat_template", "{{ bos_token }}")
                .u32("llama.context_length", 8192)
                .finish(),
        )
        .unwrap();

        assert!(md.has_chat_template);
        assert_eq!(md.context_length, Some(8192));
    }

    #[test]
    fn takes_largest_of_per_layer_head_counts() {
        let md = parse(
            minimal()
                .i32_array("llama.attention.head_count_kv", &[4, 8, 4, 8])
                .finish(),
        )
        .unwrap();

        assert_eq!(md.head_count_kv, Some(8));
    }

    #[test]
    fn key_length_wins_over_derived_head_dim() {
        let md = parse(
            minimal()
                .u32("llama.embedding_length", 4096)
                .u32("llama.attention.head_count", 32)
                .u32("llama.attention.key_length", 256)
                .finish(),
        )
        .unwrap();

        assert_eq!(md.head_dim(), Some(256));
    }

    #[test]
    fn detects_mixture_of_experts() {
        let md = parse(minimal().u32("llama.expert_count", 128).finish()).unwrap();
        assert!(md.is_moe());
    }

    #[test]
    fn rejects_non_gguf() {
        let err = parse(Cursor::new(b"NOPE____________".to_vec())).unwrap_err();
        assert!(matches!(err, ParseError::NotGguf));
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());

        let err = parse(Cursor::new(bytes)).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(1)));
    }
}
