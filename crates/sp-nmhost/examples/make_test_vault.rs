//! 本地验收辅助（仅开发/验收用途，不发布）：生成演示库。
//! 口令与条目值从环境变量读取（不在源码/参数中携带）：
//!   SP_TEST_DB_PASSWORD      库口令
//!   SP_TEST_ENTRY_PASSWORD   条目密码值
//!   SP_TEST_ENTRY_USERNAME   条目用户名（默认 demo-user）
//! 用法：cargo run -p sp-nmhost --example make_test_vault -- <kdbx路径>
//! 条目 url 固定指向 http://127.0.0.1:8080（demo 页所在 host:port）。

use std::path::PathBuf;

use sp_core::{CredentialFactors, EntryModel, KdfParams, SecureString};
use zeroize::Zeroizing;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: make_test_vault <kdbx-path>");
    let db_password = std::env::var("SP_TEST_DB_PASSWORD").expect("SP_TEST_DB_PASSWORD");
    let entry_password = std::env::var("SP_TEST_ENTRY_PASSWORD").expect("SP_TEST_ENTRY_PASSWORD");
    let username = std::env::var("SP_TEST_ENTRY_USERNAME").unwrap_or_else(|_| "demo-user".into());

    let factors = CredentialFactors::new(SecureString::new(db_password), None);
    let params = KdfParams {
        memory_kib: 64 * 1024,
        iterations: 2,
        parallelism: 1,
    };
    sp_kdbx::create_db(&PathBuf::from(&path), &factors, &params).expect("create db");
    let mut db = sp_kdbx::open_db(&PathBuf::from(&path), &factors).expect("reopen");
    sp_kdbx::add_entry(
        &mut db,
        &EntryModel {
            title: "Demo Login".into(),
            username: Some(username),
            password: Some(Zeroizing::new(entry_password)),
            url: Some("http://127.0.0.1:8080".into()),
            ..Default::default()
        },
    )
    .expect("add entry");
    sp_kdbx::save_atomic(&db, &PathBuf::from(&path), &factors).expect("save");
    println!("test vault ready: {path}");
}
