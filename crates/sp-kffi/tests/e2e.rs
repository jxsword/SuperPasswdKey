//! sp-kffi 主机侧 e2e：FFI 面语义对齐 sp-kdbx 既有测试（对照 crates/sp-kdbx/tests）。
//! 覆盖：create/open、find_entry 全字段读回、list_entry_summaries、entry_totp_now、
//!       错密码 → Credentials、keyfile 双因子、参数硬下限（64MiB）拒绝；
//!       写入面：add/edit/save/attachments/lock + 自定义字段。

use std::path::PathBuf;

use sp_kffi::{create_db, open_db, FfiCustomField, FfiEntryDraft, FfiError, FfiKdfParams};

/// RFC 6238 附录 B 参考密钥（公开规范测试向量，与 make_spike_vault 一致）
const RFC6238_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn draft(title: &str, password: &str) -> FfiEntryDraft {
    FfiEntryDraft {
        title: title.into(),
        username: Some("demo-user".into()),
        password: Some(password.into()),
        url: Some("http://127.0.0.1:8080".into()),
        notes: None,
        otp: None,
        custom_fields: vec![],
        tags: vec![],
    }
}

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

#[test]
fn mobile_default_params_dr011_fallback() {
    // DR-011：终态=新建库走 calibrate_kdf（m=64MiB/t∈[2,8]/p=4 校准至 ~1s）；
    // FfiKdfParams::default（t=4/p=4）降级为"跳过校准"兜底（fixture/测试口径），数值不变
    let d = FfiKdfParams::default();
    assert_eq!(d.memory_kib, 64 * 1024);
    assert_eq!(d.iterations, 4);
    assert_eq!(d.parallelism, 4);
    // 默认值应通过 sp-core 校验并完整走通开库链路
    let f = make_fixture("mobiledefault", d, None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db with mobile default");
    assert!(!vault.list_entry_summaries().expect("list").is_empty());
}

#[test]
fn write_path_add_edit_save_roundtrip() {
    let f = make_fixture("write", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");

    // add + 自定义字段 + 保存 + 重开读回
    let mut d = draft("New Entry", "new-pass");
    d.custom_fields = vec![FfiCustomField {
        name: "api_key".into(),
        value: "k-123".into(),
        protected: true,
    }];
    vault.add_entry(d).expect("add_entry");
    vault
        .save(
            f.db_path.to_str().unwrap().into(),
            f.db_password.clone(),
            None,
        )
        .expect("save");
    vault.lock().expect("lock");

    let vault2 = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("reopen");
    let e = vault2.find_entry("New Entry".into()).expect("find new");
    assert_eq!(e.password.as_deref(), Some("new-pass"));

    // edit（history 语义由 sp-kdbx 维护）+ 保存 + 重开
    vault2
        .edit_entry("New Entry".into(), draft("New Entry", "edited-pass"))
        .expect("edit_entry");
    vault2
        .save(
            f.db_path.to_str().unwrap().into(),
            f.db_password.clone(),
            None,
        )
        .expect("save");
    drop(vault2);

    let vault3 = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("reopen 2");
    let e = vault3.find_entry("New Entry".into()).expect("find edited");
    assert_eq!(e.password.as_deref(), Some("edited-pass"));
}

#[test]
fn save_session_uses_resident_factors_and_lock_denies() {
    // DR-012 A1：open 后凭据因子留驻 actor 线程；save_session 零凭据入参完成保存；
    // 显式锁库后因子随 actor drop 零化 → save_session 被拒（Format unavailable）
    let f = make_fixture("savesession", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");

    vault
        .add_entry(draft("Autofill Saved", "saved-pass"))
        .expect("add_entry");
    vault
        .save_session(f.db_path.to_str().unwrap().into())
        .expect("save_session with resident factors");
    vault.lock().expect("lock");
    match vault.save_session(f.db_path.to_str().unwrap().into()) {
        Err(FfiError::Format { msg }) => assert!(msg.contains("unavailable")),
        other => panic!("expected Format(unavailable) after lock, got {other:?}"),
    }

    let vault2 = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("reopen after session save");
    let e = vault2
        .find_entry("Autofill Saved".into())
        .expect("find session-saved entry");
    assert_eq!(e.password.as_deref(), Some("saved-pass"));
    assert_eq!(e.username.as_deref(), Some(f.username.as_str()));
}

#[test]
fn attachments_roundtrip() {
    let f = make_fixture("attach", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");
    let payload = vec![1u8, 2, 3, 250, 251];
    vault
        .add_attachment("Demo Login".into(), "notes.bin".into(), payload.clone())
        .expect("add_attachment");
    vault
        .save(
            f.db_path.to_str().unwrap().into(),
            f.db_password.clone(),
            None,
        )
        .expect("save");
    let got = vault
        .get_attachment("Demo Login".into(), "notes.bin".into())
        .expect("get_attachment");
    assert_eq!(got, payload);
    match vault.get_attachment("Demo Login".into(), "missing.bin".into()) {
        Err(FfiError::NotFound { .. }) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn lock_blocks_subsequent_operations() {
    let f = make_fixture("lock", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");
    vault.lock().expect("lock");
    match vault.list_entry_summaries() {
        Err(FfiError::Format { msg }) => assert!(msg.contains("unavailable")),
        other => panic!("expected Format(unavailable), got {other:?}"),
    }
    // 二次 lock：worker 已退出，发送失败 → 同样的 unavailable 语义
    match vault.lock() {
        Err(FfiError::Format { msg }) => assert!(msg.contains("unavailable")),
        other => panic!("expected Format(unavailable), got {other:?}"),
    }
}

#[test]
fn invalid_draft_rejected_parameter() {
    let f = make_fixture("baddraft", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");
    // 空标题（sp-core 校验）
    match vault.add_entry(draft("  ", "pw")) {
        Err(FfiError::Parameter { .. }) => {}
        other => panic!("expected Parameter, got {other:?}"),
    }
    // 非法 otp（非 otpauth://）
    let mut d = draft("Bad Otp", "pw");
    d.otp = Some("seed123".into());
    match vault.add_entry(d) {
        Err(FfiError::Parameter { .. }) => {}
        other => panic!("expected Parameter, got {other:?}"),
    }
    // 保留名自定义字段
    let mut d = draft("Reserved", "pw");
    d.custom_fields = vec![FfiCustomField {
        name: "Title".into(),
        value: "x".into(),
        protected: false,
    }];
    match vault.add_entry(d) {
        Err(FfiError::Parameter { .. }) => {}
        other => panic!("expected Parameter, got {other:?}"),
    }
}

#[test]
fn rank_entry_urls_matches_sp_f706_semantics() {
    use sp_kffi::rank_entry_urls;
    let urls = vec![
        Some("https://other.org/a".into()),
        Some("https://login.example.com/b".into()),
        Some("https://example.com/z".into()),   // base domain
        Some("https://evilexample.com".into()), // 子串误配拒绝
        None,                                   // 无 url 跳过
    ];
    let v = rank_entry_urls("https://login.example.com/x".into(), urls);
    assert_eq!(v, vec![1, 2]);
    // 非 http(s) 页面永不匹配
    assert!(rank_entry_urls("ftp://x".into(), vec![Some("https://x".into())]).is_empty());
}

#[test]
fn calibrate_kdf_stays_in_dr010_bounds() {
    use sp_kffi::calibrate_kdf;
    // 极小目标 → t 触底 2；超大目标 → t 封顶 8；m 恒 64MiB
    let lo = calibrate_kdf(1).expect("calibrate tiny");
    assert_eq!(lo.memory_kib, 64 * 1024);
    assert_eq!(lo.iterations, 2);
    assert_eq!(lo.parallelism, 4);
    let hi = calibrate_kdf(u32::MAX).expect("calibrate huge");
    assert_eq!(hi.iterations, 8);
}

#[test]
fn find_entry_full_returns_custom_fields_rust_only_surface() {
    // FfiEntry 读回面不含 custom_fields（v0.1 限制）；find_entry_full（仅 Rust 面）
    // 必须完整读回——desktop GUI 编辑保真的前提（design_docs/16 v1.2）
    let f = make_fixture("full", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");

    let mut d = draft("Full Entry", "pass-1");
    d.custom_fields = vec![
        FfiCustomField {
            name: "api_key".into(),
            value: "k-123".into(),
            protected: true,
        },
        FfiCustomField {
            name: "region".into(),
            value: "cn-north".into(),
            protected: false,
        },
    ];
    vault.add_entry(d).expect("add_entry");

    let full = vault.find_entry_full("Full Entry".into()).expect("full");
    // 注意：keepass-ng additional_attributes 为 HashMap 存储，读回顺序跨平台不确定——
    // 断言必须按名查找，不得依赖索引顺序（2026-10-31 linux CI 实测教训）
    assert_eq!(full.custom_fields.len(), 2);
    let api_key = full
        .custom_fields
        .iter()
        .find(|c| c.name == "api_key")
        .expect("api_key present");
    assert_eq!(api_key.value, "k-123");
    // protected 标记自 S1d-1/DR-017 起持久化（keepass-ng vendored 补丁：字段级
    // ChaCha20 ProtectedValue，与 Password/otp 同机制）
    assert!(api_key.protected);
    assert!(full.custom_fields.iter().any(|c| c.name == "region"));

    // 对照：旧 FfiEntry 面确实不含 custom_fields（回归口径不破）
    let plain = vault.find_entry("Full Entry".into()).expect("plain");
    assert_eq!(plain.password.as_deref(), Some("pass-1"));

    // 编辑保真：full 读回 → 以原 custom_fields 回写 → 重开仍在（不被清空）
    let draft2 = FfiEntryDraft {
        title: "Full Entry".into(),
        username: Some("renamed-user".into()),
        password: Some("pass-2".into()),
        url: None,
        notes: None,
        otp: None,
        custom_fields: full.custom_fields.clone(),
        tags: vec!["infra".into(), "prod".into()],
    };
    vault
        .edit_entry("Full Entry".into(), draft2)
        .expect("edit_entry");
    vault
        .save(
            f.db_path.to_str().unwrap().into(),
            f.db_password.clone(),
            None,
        )
        .expect("save");
    drop(vault);

    let vault2 = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("reopen");
    let after = vault2.find_entry_full("Full Entry".into()).expect("after");
    assert_eq!(after.password.as_deref(), Some("pass-2"));
    // 标签 roundtrip（SP-F-108：编辑写入的 tags 重开仍在）
    assert_eq!(after.tags, vec!["infra".to_string(), "prod".to_string()]);
    assert_eq!(
        after
            .custom_fields
            .iter()
            .map(|c| c.name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["api_key", "region"]),
        "custom fields must survive edit"
    );
}

#[test]
fn delete_entry_rust_only_surface_removes_entry() {
    let f = make_fixture("delete", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");
    vault.add_entry(draft("Doomed", "p")).expect("add");

    vault.delete_entry("Doomed".into()).expect("delete_entry");
    match vault.find_entry("Doomed".into()) {
        Err(FfiError::NotFound { .. }) => {}
        other => panic!("expected NotFound after delete, got {other:?}"),
    }
    // 不存在的条目删除 → NotFound
    match vault.delete_entry("Nope".into()) {
        Err(FfiError::NotFound { .. }) => {}
        other => panic!("expected NotFound for missing entry, got {other:?}"),
    }
}

#[test]
fn list_attachments_rust_only_surface() {
    // GUI 附件管理（S1b-2b）：list_attachments 列名（仅 Rust 面）
    let f = make_fixture("attlist", FfiKdfParams::default(), None);
    let vault = open_db(
        f.db_path.to_str().unwrap().into(),
        f.db_password.clone(),
        None,
    )
    .expect("open_db");
    vault.add_entry(draft("WithFiles", "p")).expect("add");
    vault
        .add_attachment("WithFiles".into(), "a.txt".into(), b"alpha".to_vec())
        .expect("add att 1");
    vault
        .add_attachment("WithFiles".into(), "b.bin".into(), vec![1, 2, 3])
        .expect("add att 2");

    let mut names = vault
        .list_attachments("WithFiles".into())
        .expect("list_attachments");
    names.sort();
    assert_eq!(names, vec!["a.txt".to_string(), "b.bin".to_string()]);
    // 无附件条目 → 空表
    vault.add_entry(draft("NoFiles", "p")).expect("add 2");
    assert!(vault
        .list_attachments("NoFiles".into())
        .expect("empty list")
        .is_empty());
}
