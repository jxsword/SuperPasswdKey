//! sp-nmhost 库目标：协议/会话/匹配引擎供单测与 cargo-fuzz 复用（bin 入口见 main.rs）。
//! 设计见 design_docs/10/11/12（DR-006）；step7 起动作分发 trait 化 + NM 循环库化，
//! 供 sp-desktop 单进程嵌入复用（DR-013，design_docs/15）。

pub mod actions;
pub mod config;
pub mod match_url;
pub mod nm_loop;
pub mod origin;
pub mod protocol;
pub mod session;
