//! sp 库目标：供 cargo-fuzz 与外部集成测试复用（bin 入口见 main.rs）。
//! 模块划分与文档见 design_docs/03/04。

pub mod commands;
pub mod config;
pub mod export;
pub mod f8;
pub mod importer;
pub mod integrity;
pub mod spawner;
