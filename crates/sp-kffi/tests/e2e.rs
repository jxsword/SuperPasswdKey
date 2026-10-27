//! sp-kffi 主机侧 e2e：FFI 面语义对齐 sp-kdbx 既有测试（对照 crates/sp-kdbx/tests）。
//! 覆盖：create/open、find_entry 全字段读回、list_entry_summaries、entry_totp_now、
//!       错密码 → Credentials、keyfile 双因子、参数硬下限（64MiB）拒绝。

use std::path::PathBuf;

use sp_kffi::{create_db, open_db, FfiError, FfiKdfParams};

/// RFC 6238 附录 B 参考密钥（公开规范测试向量，与 make_spike_vault 一致）
const RFC6238_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

struct Fixture {
    db_path: PathBuf,
    db_password: String,
    entry_password: String,
    username: String,
}

fn make_fixture(tag: &str, params: FfiKdfParams, keyfile: Option<&PathBuf>) -> Fixture {
    let dir = std::env::temp_dir().join(format!("sp-kffi-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let db_path = dir.join("test.kdbx");
    let db_password = env_or("SP_TEST_DB_PASSWORD", "test-db-pass");
    let entry_password = env_or("SP_TEST_ENTRY_PASSWORD", "test-entry-pass");
    let username = env_or("SP_TEST_ENTRY_USERNAME", "demo-user");

    if let Some(kf) = keyfile {
        sp_kdbx::generate_keyfile(kf).expect("generate_keyfile");
    }
    create_db(
        db_path.to_str().unwrap().to_string(),
        db_password.clone(),
        keyfile.map(|p| p.to_str().unwrap().to_string()),
        params,
    )
    .expect("create_db");

    // 条目填充走 sp-kdbx 原生 API（add_entry 不在 spike FFI 面内），与桌面语义同源
    let factors = sp_core::CredentialFactors::new(
        sp_core::SecureString::new(db_password.clone()),
        keyfile.cloned(),
    );
    let mut db = sp_kdbx::open_db(&db_path, &factors).expect("seed open");
    sp_kdbx::add_entry(
        &mut db,
        &sp_core::EntryModel {
            title: "Demo Login".into(),
            username: Some(username.clone()),
            password: Some(zeroize::Zeroizing::new(entry_password.clone())),
            url: Some("http://127.0.0.1:8080".into()),
            ..Default::default()
        },
    )
    .expect("add entry 1");
    sp_kdbx::add_entry(
        &mut db,
        &sp_core::EntryModel {
            title: "Demo OTP".into(),
            username: Some("demo-user".into()),
            otp: Some(format!(
                "otpauth://totp/Demo%20OTP?secret={RFC6238_SECRET_B32}&issuer=SuperPasswdKey&digits=6&period=30"
            )),
            ..Default::default()
        },
    )
    .expect("add entry 2");
    sp_kdbx::save_atomic(&db, &db_path, &factors).expect("seed save");

    Fixture {
        db_path,
        db_password,
        entry_password,
        username,
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.db_path.parent().unwrap());
    }
}

#[test]
fn default_params_pass_validation_floor() {
    let f = make_fixture("main", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");

    // find_entry 全字段读回
    let e = vault.find_entry("Demo Login".into()).expect("find_entry");
    assert_eq!(e.title, "Demo Login");
    assert_eq!(e.username.as_deref(), Some(f.username.as_str()));
    assert_eq!(e.password.as_deref(), Some(f.entry_password.as_str()));
    assert_eq!(e.url.as_deref(), Some("http://127.0.0.1:8080"));

    // 摘要不含密码、has_otp 正确
    let mut sums = vault.list_entry_summaries().expect("list_entry_summaries");
    sums.sort_by(|a, b| a.title.cmp(&b.title));
    assert_eq!(sums.len(), 2);
    assert_eq!(sums[0].title, "Demo Login");
    assert_eq!(sums[0].username.as_deref(), Some(f.username.as_str()));
    assert!(!sums[0].has_otp);
    assert_eq!(sums[1].title, "Demo OTP");
    assert!(sums[1].has_otp);

    // TOTP：6 位数字码（时间相关，断言形状）
    let code = vault
        .entry_totp_now("Demo OTP".into())
        .expect("entry_totp_now");
    assert_eq!(code.len(), 6, "totp code shape: {code}");
    assert!(code.chars().all(|c| c.is_ascii_digit()));

    // NotFound 语义
    match vault.find_entry("missing".into()) {
        Err(FfiError::NotFound { .. }) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn wrong_password_is_credentials_error() {
    let f = make_fixture("wrongpw", FfiKdfParams::default(), None);
    match open_db(
        f.db_path.to_str().unwrap().into(),
        "definitely-wrong".into(),
        None,
    ) {
        Err(FfiError::Credentials) => {}
        other => panic!("expected Credentials, got {other:?}"),
    }
}

#[test]
fn keyfile_dual_factor_opens() {
    let dir = std::env::temp_dir().join(format!("sp-kffi-kf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let kf = dir.join("test.keyfile");
    let f = make_fixture("keyfile", FfiKdfParams::default(), Some(&kf));

    // 双因子正确 → 成功
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        Some(kf.to_str().unwrap().into()),
    )
    .expect("open with keyfile");
    assert!(!vault.list_entry_summaries().expect("list").is_empty());

    // 缺 keyfile → Credentials（凭据语义不细化）
    match open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    ) {
        Err(FfiError::Credentials) => {}
        other => panic!("expected Credentials, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn below_memory_floor_rejected() {
    let dir = std::env::temp_dir().join(format!("sp-kffi-floor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let db_path = dir.join("floor.kdbx");
    match create_db(
        db_path.to_str().unwrap().into(),
        "pw".into(),
        None,
        FfiKdfParams {
            memory_kib: 32 * 1024,
            iterations: 2,
            parallelism: 1,
        },
    ) {
        Err(FfiError::Parameter { .. }) => {}
        other => panic!("expected Parameter, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
