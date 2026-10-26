//! sp-nmhost 库目标：协议/会话/匹配引擎供单测与 cargo-fuzz 复用（bin 入口见 main.rs）。
//! 设计见 design_docs/10/11/12（DR-006）。

pub mod actions;
pub mod config;
pub mod match_url;
pub mod protocol;
pub mod session;
