//! 帧编解码（SP-F-702，design_docs/11 §1-2）：
//! 32 位本机字节序长度前缀 + UTF-8 JSON 信封 `{v, codec, payload}`。
//! 上限：host→浏览器 1MB（Chrome 官方限制）；浏览器→host 平台上限 64MiB，host 自防 8MiB。
//! codec 为 P1(plain)/P2(naclbox) 适配器切换点（DR-006）：本实现仅 plain，
//! 业务层不得感知 codec。

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u64 = 1;
/// host→浏览器单帧上限（Chrome 限制 1MB，取保守整值）
pub const MAX_FRAME_OUT: usize = 1_000_000;
/// 浏览器→host 单帧自防上限（合法负载远小于此，防 DoS）
pub const MAX_FRAME_IN: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum ProtocolError {
    Io(std::io::Error),
    FrameTooLarge(usize),
    Truncated,
    InvalidUtf8,
    Json(serde_json::Error),
    UnsupportedVersion(u64),
    UnsupportedCodec(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::Io(e) => write!(f, "io error: {e}"),
            ProtocolError::FrameTooLarge(n) => write!(f, "frame too large: {n} bytes"),
            ProtocolError::Truncated => write!(f, "truncated frame"),
            ProtocolError::InvalidUtf8 => write!(f, "frame is not valid utf-8"),
            ProtocolError::Json(e) => write!(f, "invalid json: {e}"),
            ProtocolError::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            ProtocolError::UnsupportedCodec(c) => write!(f, "unsupported codec: {c}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// 消息信封：帧内 JSON 的最外层（design_docs/11 §2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u64,
    pub codec: String,
    pub payload: Value,
}

impl Envelope {
    pub fn plain(payload: Value) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            codec: "plain".into(),
            payload,
        }
    }
}

fn check_envelope(env: &Envelope) -> Result<(), ProtocolError> {
    if env.v != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(env.v));
    }
    match env.codec.as_str() {
        "plain" => Ok(()),
        "naclbox" => Err(ProtocolError::UnsupportedCodec(
            "naclbox is reserved for P2 (not implemented); use plain".into(),
        )),
        other => Err(ProtocolError::UnsupportedCodec(other.to_string())),
    }
}

/// 从原始帧字节解码信封（fuzz 入口；非法输入返回错误，绝不 panic）
pub fn decode_frame(data: &[u8]) -> Result<Envelope, ProtocolError> {
    if data.len() > MAX_FRAME_IN {
        return Err(ProtocolError::FrameTooLarge(data.len()));
    }
    let s = std::str::from_utf8(data).map_err(|_| ProtocolError::InvalidUtf8)?;
    let env: Envelope = serde_json::from_str(s).map_err(ProtocolError::Json)?;
    check_envelope(&env)?;
    Ok(env)
}

pub fn encode_envelope(env: &Envelope) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(env).map_err(ProtocolError::Json)
}

/// 读一帧：帧边界处干净 EOF → None（进程正常退出）；半帧 → Truncated。
pub fn read_frame(r: &mut dyn Read) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut len_buf = [0u8; 4];
    let mut filled = 0;
    while filled < 4 {
        let n = r.read(&mut len_buf[filled..]).map_err(ProtocolError::Io)?;
        if n == 0 {
            if filled == 0 {
                return Ok(None);
            }
            return Err(ProtocolError::Truncated);
        }
        filled += n;
    }
    // 本机字节序：支持平台（Windows/Linux x64）均为小端（design_docs/11 §1）
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_FRAME_IN {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut buf = vec![0u8; len];
    if let Err(e) = r.read_exact(&mut buf) {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Err(ProtocolError::Truncated);
        }
        return Err(ProtocolError::Io(e));
    }
    Ok(Some(buf))
}

/// 写一帧；超上限拒绝（Chrome 会丢弃超限消息）
pub fn write_frame(w: &mut dyn Write, data: &[u8]) -> Result<(), ProtocolError> {
    if data.len() > MAX_FRAME_OUT {
        return Err(ProtocolError::FrameTooLarge(data.len()));
    }
    let len = u32::try_from(data.len()).map_err(|_| ProtocolError::FrameTooLarge(data.len()))?;
    w.write_all(&len.to_le_bytes()).map_err(ProtocolError::Io)?;
    w.write_all(data).map_err(ProtocolError::Io)?;
    w.flush().map_err(ProtocolError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn roundtrip(env: &Envelope) -> Result<Envelope, ProtocolError> {
        let bytes = encode_envelope(env)?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        framed.extend_from_slice(&bytes);
        let mut cursor = std::io::Cursor::new(framed);
        let frame = read_frame(&mut cursor)?.expect("frame present");
        decode_frame(&frame)
    }

    #[test]
    fn envelope_roundtrip() {
        let env = Envelope::plain(json!({"action": "sp.ping", "requestID": "abcd1234"}));
        let back = roundtrip(&env).expect("roundtrip ok");
        assert_eq!(back.v, 1);
        assert_eq!(back.codec, "plain");
        assert_eq!(back.payload["action"], "sp.ping");
        assert_eq!(back.payload["requestID"], "abcd1234");
    }

    #[test]
    fn empty_payload_frame() {
        let env = Envelope::plain(json!({}));
        assert!(roundtrip(&env).is_ok());
    }

    #[test]
    fn clean_eof_returns_none() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        assert!(read_frame(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn truncated_length_prefix_rejected() {
        let mut cursor = std::io::Cursor::new(vec![0u8, 0, 1]);
        assert!(matches!(
            read_frame(&mut cursor),
            Err(ProtocolError::Truncated)
        ));
    }

    #[test]
    fn truncated_body_rejected() {
        let mut framed = 100u32.to_le_bytes().to_vec();
        framed.extend_from_slice(b"short");
        let mut cursor = std::io::Cursor::new(framed);
        assert!(matches!(
            read_frame(&mut cursor),
            Err(ProtocolError::Truncated)
        ));
    }

    #[test]
    fn oversized_input_frame_rejected() {
        let mut cursor = std::io::Cursor::new((MAX_FRAME_IN as u32 + 1).to_le_bytes().to_vec());
        assert!(matches!(
            read_frame(&mut cursor),
            Err(ProtocolError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn oversized_output_frame_rejected() {
        let big = vec![0u8; MAX_FRAME_OUT + 1];
        assert!(matches!(
            write_frame(&mut std::io::sink(), &big),
            Err(ProtocolError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn invalid_utf8_rejected() {
        let data = vec![0xff, 0xfe, 0x00];
        assert!(matches!(
            decode_frame(&data),
            Err(ProtocolError::InvalidUtf8)
        ));
    }

    #[test]
    fn arbitrary_bytes_never_panic() {
        // fuzz 目标的种子语义：任意字节流返回 Err 而非崩溃
        for data in [
            vec![],
            vec![0u8; 4],
            b"null".to_vec(),
            b"{}".to_vec(),
            b"{\"v\":1}".to_vec(),
            b"{\"v\":1,\"codec\":\"plain\",\"payload\":null}".to_vec(),
            vec![0x80; 64],
        ] {
            let _ = decode_frame(&data);
        }
    }

    #[test]
    fn unsupported_version_rejected() {
        let env = Envelope {
            v: 2,
            codec: "plain".into(),
            payload: json!({}),
        };
        assert!(matches!(
            roundtrip(&env),
            Err(ProtocolError::UnsupportedVersion(2))
        ));
    }

    #[test]
    fn naclbox_reserved_rejected() {
        let env = Envelope {
            v: 1,
            codec: "naclbox".into(),
            payload: json!({}),
        };
        assert!(matches!(
            roundtrip(&env),
            Err(ProtocolError::UnsupportedCodec(_))
        ));
    }

    #[test]
    fn unknown_codec_rejected() {
        let env = Envelope {
            v: 1,
            codec: "rot13".into(),
            payload: json!({}),
        };
        assert!(matches!(
            roundtrip(&env),
            Err(ProtocolError::UnsupportedCodec(_))
        ));
    }
}
