//! 本地/模拟器验收辅助（仅开发/验收用途，不发布）：生成 Android spike 测试库。
//! 口令与条目值从环境变量读取（不在源码/参数中携带）：
//!   SP_TEST_DB_PASSWORD      库口令（必填）
//!   SP_TEST_ENTRY_PASSWORD   条目密码值（必填）
//!   SP_TEST_ENTRY_USERNAME   条目用户名（默认 demo-user）
//!   SP_TEST_ENTRY_URL        条目 url（默认 http://127.0.0.1:8080；Autofill 演示传应用包名做裸 host 匹配）
//!   SP_TEST_KEYFILE          可选：keyfile 输出路径（设置则生成 keyfile 并启用双因子）
//! 用法：cargo run -p sp-kffi --example make_spike_vault -- <kdbx路径>
//! 产出：两个条目——"Demo Login"（基础断言面）与 "Demo OTP"（RFC 6238 参考密钥，TOTP 面）。
//! KDF 用默认参数 m=64MiB/t=8/p=4（KDF 实测口径，SP-N-301/302）。

use std::path::PathBuf;

use sp_core::{CredentialFactors, EntryModel, KdfParams, SecureString};
use zeroize::Zeroizing;

/// RFC 6238 附录 B 参考密钥（ASCII "12345678901234567890"）的 Base32 编码——公开规范测试向量
const RFC6238_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: make_spike_vault <kdbx-path>");
    let db_password = std::env::var("SP_TEST_DB_PASSWORD").expect("SP_TEST_DB_PASSWORD");
    let entry_password = std::env::var("SP_TEST_ENTRY_PASSWORD").expect("SP_TEST_ENTRY_PASSWORD");
    let username = std::env::var("SP_TEST_ENTRY_USERNAME").unwrap_or_else(|_| "demo-user".into());
    let entry_url =
        std::env::var("SP_TEST_ENTRY_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());

    let keyfile = std::env::var("SP_TEST_KEYFILE").ok().map(PathBuf::from);
    if let Some(kf) = &keyfile {
        sp_kdbx::generate_keyfile(kf).expect("generate keyfile");
    }
    let factors = CredentialFactors::new(SecureString::new(db_password), keyfile);
    let params = KdfParams::default();
    let path = PathBuf::from(&path);

    sp_kdbx::create_db(&path, &factors, &params).expect("create db");
    let mut db = sp_kdbx::open_db(&path, &factors).expect("reopen");
    sp_kdbx::add_entry(
        &mut db,
        &EntryModel {
            title: "Demo Login".into(),
            username: Some(username),
            password: Some(Zeroizing::new(entry_password)),
            url: Some(entry_url),
            ..Default::default()
        },
    )
    .expect("add entry 1");
    sp_kdbx::add_entry(
        &mut db,
        &EntryModel {
            title: "Demo OTP".into(),
            username: Some("demo-user".into()),
            otp: Some(format!(
                "otpauth://totp/Demo%20OTP?secret={RFC6238_SECRET_B32}&issuer=SuperPasswdKey&digits=6&period=30"
            )),
            ..Default::default()
        },
    )
    .expect("add entry 2");
    sp_kdbx::save_atomic(&db, &path, &factors).expect("save");
    println!("spike vault ready: {}", path.display());
}
