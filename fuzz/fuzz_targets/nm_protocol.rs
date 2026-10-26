#![no_main]
//! native messaging 协议 fuzz（SP-F-702 / design_docs/11 §8）：
//! ① 任意字节流经 `decode_frame` 不得 panic（帧边界/非法 UTF-8/未知版本与 codec）；
//! ② 解码出的 payload 进动作分发（锁定态会话，无库 IO）不得 panic。

use libfuzzer_sys::fuzz_target;
use sp_nmhost::session::Session;
use sp_nmhost::{actions, protocol};

fuzz_target!(|data: &[u8]| {
    if let Ok(env) = protocol::decode_frame(data) {
        let session = Session::new(300, None);
        let _ = actions::handle(&session, &env.payload);
    }
});
