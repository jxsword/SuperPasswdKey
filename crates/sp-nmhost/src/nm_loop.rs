//! NM 请求循环（step7 库化，design_docs/15）：逐帧 读入 → 分发 → 响应信封写出。
//! sp-nmhost bin 与 sp-desktop 单进程嵌入共用本循环（DR-013 单进程架构），
//! 协议编解码不复制（protocol.rs 单源）。

use std::io::{BufReader, Read, Write};
use std::sync::{Mutex, PoisonError};

use serde_json::{json, Value};

use crate::actions::ActionError;
use crate::protocol::{decode_frame, encode_envelope, read_frame, write_frame, Envelope};

/// serve 循环结束原因。
#[derive(Debug)]
pub enum ServeEnd {
    /// stdin 干净 EOF（浏览器关闭 native messaging 端口；写失败视为同类）
    Eof,
    /// 协议错误（半帧/非法 UTF-8/坏 JSON/未知版本或 codec）
    Protocol(crate::protocol::ProtocolError),
}

/// 请求循环：每帧一个请求（design_docs/11 §3）；响应信封由本函数统一整形
/// （action/requestID/success/data|error），分发方只关心 payload → data。
/// 输出经 `out` 互斥锁串行化——watcher 线程的 sp.event 帧与响应帧不得交错。
pub fn serve<R: Read, W: Write, F: FnMut(&Value) -> Result<Value, ActionError>>(
    input: R,
    out: &Mutex<W>,
    mut dispatch: F,
) -> ServeEnd {
    let mut input = BufReader::new(input);
    loop {
        let frame = match read_frame(&mut input) {
            Ok(Some(f)) => f,
            Ok(None) => return ServeEnd::Eof,
            Err(e) => return ServeEnd::Protocol(e),
        };
        let env = match decode_frame(&frame) {
            Ok(e) => e,
            Err(e) => return ServeEnd::Protocol(e),
        };
        let request_id = env.payload.get("requestID").cloned().unwrap_or(Value::Null);
        let action = env
            .payload
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let response = match dispatch(&env.payload) {
            Ok(data) => json!({
                "action": action,
                "requestID": request_id,
                "success": true,
                "data": data,
            }),
            Err(e) => json!({
                "action": action,
                "requestID": request_id,
                "success": false,
                "error": {"code": e.code, "message": e.message},
            }),
        };
        let bytes = match encode_envelope(&Envelope::plain(response)) {
            Ok(b) => b,
            Err(e) => return ServeEnd::Protocol(e),
        };
        let mut w = out.lock().unwrap_or_else(PoisonError::into_inner);
        if write_frame(&mut *w, &bytes).is_err() {
            // 写失败 = 端口已关闭，等价 EOF
            return ServeEnd::Eof;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    /// 用内存游标模拟一次完整握手：发一帧请求，收一帧响应 payload（无帧则 None）。
    fn handshake(payload: Value) -> Option<Result<Value, ServeEnd>> {
        let body = serde_json::to_string(&Envelope::plain(payload)).expect("json");
        let mut input = Vec::new();
        input.extend_from_slice(&(body.len() as u32).to_le_bytes());
        input.extend_from_slice(body.as_bytes());
        let out: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        let end = serve(Cursor::new(input), &out, |p| {
            match p.get("action").and_then(Value::as_str) {
                Some("sp.ping") => Ok(json!({ "locked": true })),
                _ => Err(ActionError::new("badrequest", "nope")),
            }
        });
        assert!(matches!(end, ServeEnd::Eof), "clean EOF expected");
        let mut buf = out.lock().unwrap_or_else(PoisonError::into_inner);
        if buf.is_empty() {
            return None;
        }
        let mut cursor = Cursor::new(std::mem::take(&mut *buf));
        let frame = match read_frame(&mut cursor) {
            Ok(Some(f)) => f,
            Ok(None) => return None,
            Err(_) => return None,
        };
        match decode_frame(&frame) {
            Ok(env) => Some(Ok(env.payload)),
            Err(_) => Some(Err(end)),
        }
    }

    #[test]
    fn serve_roundtrips_request_to_enveloped_response() {
        let resp = handshake(json!({"action": "sp.ping", "requestID": "r1"}))
            .expect("response frame")
            .expect("decode");
        assert_eq!(resp["action"], "sp.ping");
        assert_eq!(resp["requestID"], "r1");
        assert_eq!(resp["success"], true);
        assert_eq!(resp["data"]["locked"], true);
    }

    #[test]
    fn serve_wraps_dispatch_error() {
        let resp = handshake(json!({"action": "sp.nope", "requestID": null}))
            .expect("response frame")
            .expect("decode");
        assert_eq!(resp["success"], false);
        assert_eq!(resp["error"]["code"], "badrequest");
    }

    #[test]
    fn serve_empty_input_is_clean_eof() {
        let out: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        let end = serve(Cursor::new(Vec::new()), &out, |_| Ok(json!({})));
        assert!(matches!(end, ServeEnd::Eof));
        assert!(out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty());
    }

    #[test]
    fn serve_bad_frame_ends_with_protocol_error() {
        // 半帧：长度前缀声明 10 字节只给 3 字节
        let mut input = 10u32.to_le_bytes().to_vec();
        input.extend_from_slice(b"abc");
        let out: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        let end = serve(Cursor::new(input), &out, |_| Ok(json!({})));
        assert!(matches!(end, ServeEnd::Protocol(_)));
    }
}
