//! H1 round-trip 互操作测试（design_docs/02 §3）。
//!
//! 覆盖：sp(keepass_ng) 建 → keepassxc-cli 验 → keepassxc-cli 改 → sp 验。
//! Windows 版 keepassxc-cli 不接受管道密码（需 TTY，实测"没有设置密钥"中止），
//! 故 keepassxc-cli 步骤使用 keyfile-only 解锁（`-k <keyfile>`）；密码+keyfile
//! 复合凭据的保存/打开往返在 keepass_ng 侧验证（KDBX4 凭据组合是格式标准行为）。
//! fixture 密码 `sp-test-only` 仅测试用途（SRS §5，可入仓）。

use std::path::PathBuf;
use std::process::{Command, Stdio};

use keepass_ng::db::Attachment;
use keepass_ng::db::{group_add_child, rc_refcell_node, Database, Entry, Group, Node};
use keepass_ng::{Argon2Version, DatabaseConfig, DatabaseKey, KdfConfig};

/// 降低参数保测试速度（产品默认参数策略见 sp-core::keypolicy；memory 单位=字节）
const TEST_KDF_MEMORY_BYTES: u64 = 16 * 1024 * 1024;

/// keepassxc-cli 定位：`SP_KEEPASSXC_CLI` 环境变量覆盖 → 平台默认路径
/// （Windows: Program Files 安装位置；Linux: apt 安装位置 /usr/bin）
fn keepassxc_cli_path() -> String {
    if let Ok(p) = std::env::var("SP_KEEPASSXC_CLI") {
        if !p.is_empty() {
            return p;
        }
    }
    if cfg!(windows) {
        r"C:\Program Files\KeePassXC\keepassxc-cli.exe".to_string()
    } else {
        "/usr/bin/keepassxc-cli".to_string()
    }
}

fn kxc(args: &[&str]) -> String {
    let kxc_path = keepassxc_cli_path();
    assert!(
        std::path::Path::new(&kxc_path).is_file(),
        "keepassxc-cli not found at {kxc_path} (install KeePassXC >= 2.7 or set SP_KEEPASSXC_CLI)"
    );
    let mut cmd = Command::new(&kxc_path);
    // 测试库均为 keyfile-only：跳过密码提示（Windows CLI 无管道密码输入能力）
    // 注意：全局选项必须放在子命令之后（clap 语义）
    let mut full_args: Vec<String> = vec![args[0].to_string(), "--no-password".to_string()];
    full_args.extend(args[1..].iter().map(|s| s.to_string()));
    cmd.args(&full_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd.output().expect("keepassxc-cli output");
    assert!(
        out.status.success(),
        "keepassxc-cli {args:?} failed: status={:?} stderr={} stdout={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// 以 keepass_ng 创建测试库：keyfile-only（供 keepassxc-cli 免 TTY 解锁）
fn create_test_db(path: &PathBuf, keyfile_path: &PathBuf) {
    let config = DatabaseConfig {
        kdf_config: KdfConfig::Argon2id {
            iterations: 3,
            memory: TEST_KDF_MEMORY_BYTES,
            parallelism: 2,
            version: Argon2Version::Version13,
        },
        ..DatabaseConfig::default()
    };
    let db = Database::new(config);

    let mut entry = Entry::default();
    entry.set_title(Some("sp-roundtrip"));
    entry.set_username(Some("sp-user"));
    entry.set_password(Some("sp-secret-value"));
    entry.set_url(Some("https://example.com"));
    entry.set_notes(Some("notes line\nsecond line"));
    // TOTP 自定义属性（KeePass 约定编码，KeePassXC 可识别）
    // KeePassXC "otp" 属性仅接受 otpauth:// URL 或 KeeOtp "key=" 格式
    // （源码 Totp::parseSettings：LEGACY 分支的 seed 存独立 "seed" 属性）
    let otp_url = "otpauth://totp/sp-roundtrip?secret=JBSWY3DPEHPK3PXP&period=30&digits=6";
    entry.set_raw_otp_value(Some(otp_url));
    assert_eq!(entry.get_raw_otp_value().unwrap(), otp_url);
    // 附件（KDBX4 binaries）
    entry.attachments.insert(
        "sp-attachment.txt".to_string(),
        Attachment {
            data: keepass_ng::db::Value::unprotected(b"attachment-payload".to_vec()),
        },
    );
    group_add_child(&db.root, rc_refcell_node(entry), 0).expect("add entry");

    let keyfile = std::fs::File::open(keyfile_path).expect("keyfile exists");
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(keyfile))
        .expect("build key");
    let mut out = std::fs::File::create(path).expect("create db file");
    db.save(&mut out, key).expect("save kdbx4");
}

/// 生成官方 XML KeyFile v1.0 格式密钥文件（32B 熵，Base64 编码）。
/// 不用裸 32B legacy 格式：keepass_ng 直接用字节作密钥，而 KeePassXC 按
/// SHA-256(内容) 派生——语义分歧会导致 HMAC 不匹配（实测）。XML 格式双方一致。
fn gen_keyfile(path: &PathBuf) {
    use base64::Engine as _;
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).expect("os rng");
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<KeyFile>
\t<Meta>
\t\t<Version>1.00</Version>
\t</Meta>
\t<Key>
\t\t<Data>{b64}</Data>
\t</Key>
</KeyFile>
"
    );
    std::fs::write(path, xml).expect("write keyfile");
}

/// 复合凭据（密码+keyfile）的保存/打开往返（keepass_ng 侧）
#[test]
fn composite_credentials_save_open_roundtrip() {
    let dir = std::env::temp_dir().join(format!("sp-composite-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("composite.kdbx");
    let keyfile_path = dir.join("composite.keyfile");
    gen_keyfile(&keyfile_path);

    let config = DatabaseConfig {
        kdf_config: KdfConfig::Argon2id {
            iterations: 3,
            memory: TEST_KDF_MEMORY_BYTES,
            parallelism: 2,
            version: Argon2Version::Version13,
        },
        ..DatabaseConfig::default()
    };
    let db = Database::new(config);
    let mut entry = Entry::default();
    entry.set_title(Some("composite"));
    entry.set_password(Some("pw-protected-value"));
    group_add_child(&db.root, rc_refcell_node(entry), 0).expect("add entry");

    // 保存：密码 + keyfile 复合凭据
    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_password("sp-test-only")
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .expect("build key");
    let mut out = std::fs::File::create(&db_path).expect("create file");
    db.save(&mut out, key).expect("save with composite key");

    // 打开：正确凭据 → 成功
    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_password("sp-test-only")
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .expect("build key");
    let mut f = std::fs::File::open(&db_path).unwrap();
    let reopened = Database::open(&mut f, key).expect("open with composite key");
    let root = reopened.root.borrow();
    let group = root.downcast_ref::<Group>().expect("root group");
    let ok = group.entries().iter().any(|n| {
        n.borrow()
            .downcast_ref::<Entry>()
            .map(|e| e.get_password() == Some("pw-protected-value"))
            .unwrap_or(false)
    });
    assert!(ok, "composite round-trip preserved entry");

    // 打开：错误密码 → 失败
    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let bad = DatabaseKey::new()
        .with_password("wrong-password")
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .expect("build key");
    let mut f = std::fs::File::open(&db_path).unwrap();
    assert!(
        Database::open(&mut f, bad).is_err(),
        "wrong password must fail"
    );

    // 保留产物供手动排查
}

/// 诊断：保存→重开 后 otp 字段是否存续（keepass_ng 实验 save 的关键验证）
#[test]
fn debug_otp_persists_after_save_reload() {
    let dir = std::env::temp_dir().join(format!("sp-otpcheck-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("otp.kdbx");
    let keyfile_path = dir.join("otp.keyfile");
    gen_keyfile(&keyfile_path);

    let config = DatabaseConfig {
        kdf_config: KdfConfig::Argon2id {
            iterations: 3,
            memory: TEST_KDF_MEMORY_BYTES,
            parallelism: 2,
            version: Argon2Version::Version13,
        },
        ..DatabaseConfig::default()
    };
    let db = Database::new(config);
    let mut entry = Entry::default();
    entry.set_title(Some("otpcheck"));
    entry.set_raw_otp_value(Some("JBSWY3DPEHPK3PXP?period=30&digits=6"));
    group_add_child(&db.root, rc_refcell_node(entry), 0).expect("add");

    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .unwrap();
    let mut out = std::fs::File::create(&db_path).unwrap();
    db.save(&mut out, key).expect("save");

    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .unwrap();
    let mut f = std::fs::File::open(&db_path).unwrap();
    let xml = Database::get_xml(&mut f, key).expect("get xml");
    let xml_str = String::from_utf8_lossy(&xml).into_owned();
    let idx = xml_str.find("otp").expect("otp in xml");
    println!(
        "XML around otp: {}",
        &xml_str[idx.saturating_sub(120)..idx + 160]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// keepassxc-cli 互操作（keyfile-only 解锁）
#[test]
fn roundtrip_sp_to_keepassxc_and_back() {
    let dir = std::env::temp_dir().join(format!("sp-roundtrip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.kdbx");
    let keyfile_path = dir.join("test.keyfile");
    gen_keyfile(&keyfile_path);
    create_test_db(&db_path, &keyfile_path);
    let db_str = db_path.to_string_lossy().to_string();
    let kf = keyfile_path.to_string_lossy().to_string();

    // ---- 阶段 1：keepassxc-cli 验证 sp 写入库 ----
    let show = |args: &[&str]| {
        let mut a = vec![
            "show",
            "-q",
            "-k",
            kf.as_str(),
            db_str.as_str(),
            "sp-roundtrip",
        ];
        a.extend_from_slice(args);
        kxc(&a)
    };
    let out = show(&["-a", "UserName"]);
    assert!(out.contains("sp-user"), "username via keepassxc: {out}");
    let out = show(&["-a", "Password", "-s"]);
    assert!(
        out.contains("sp-secret-value"),
        "password via keepassxc: {out}"
    );
    let out = show(&["-a", "URL"]);
    assert!(out.contains("example.com"), "url via keepassxc: {out}");

    // TOTP：keepassxc-cli show -t 直接生成当前码（6 位数字）
    let totp_out = kxc(&[
        "show",
        "-q",
        "-t",
        "-k",
        kf.as_str(),
        db_str.as_str(),
        "sp-roundtrip",
    ]);
    assert!(
        totp_out.trim().chars().all(|c| c.is_ascii_digit()) && totp_out.trim().len() == 6,
        "totp via keepassxc: {totp_out}"
    );

    // 附件导出：attachment-export [-f] <db> <entry> <attachment> <output>
    let att_path = dir.join("attachment-out.txt");
    kxc(&[
        "attachment-export",
        "-q",
        "-k",
        kf.as_str(),
        db_str.as_str(),
        "sp-roundtrip",
        "sp-attachment.txt",
        att_path.to_string_lossy().as_ref(),
    ]);
    let payload = std::fs::read(&att_path).expect("attachment exported");
    assert_eq!(payload, b"attachment-payload", "attachment round-trip");

    // ---- 阶段 2：keepassxc-cli 修改库，sp(keepass_ng) 读回 ----
    kxc(&[
        "add",
        "-q",
        "-k",
        kf.as_str(),
        db_str.as_str(),
        "kxc-added",
        "-u",
        "kxc-user",
        "-g",
    ]);
    // kxc 生成的密码非空即可（值由 kxc 生成），从 sp 侧读回
    let keyfile = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(keyfile))
        .unwrap();
    let mut f = std::fs::File::open(&db_path).unwrap();
    let db = Database::open(&mut f, key).expect("reopen db modified by keepassxc");
    let found = {
        let root = db.root.borrow();
        let group = root.downcast_ref::<Group>().expect("root group");
        group.entries().iter().any(|node| {
            let e = node.borrow();
            e.downcast_ref::<Entry>()
                .map(|e| {
                    e.get_title() == Some("kxc-added")
                        && e.get_username() == Some("kxc-user")
                        && e.get_password().map(|p| !p.is_empty()).unwrap_or(false)
                })
                .unwrap_or(false)
        })
    };
    assert!(found, "entry added by keepassxc visible via keepass_ng");

    let _ = std::fs::remove_dir_all(&dir);
}

/// S1d-1/DR-017：受保护自定义属性互操作——sp(keepass_ng vendored patch) 写
/// ProtectedValue 自定义属性 → keepassxc-cli 无 `-s` 掩码、`-s` 解掩码读出 →
/// sp 重开后保护位与值保真（KDBX4 格式原生语义，KeePassXC 掩码互认）。
#[test]
fn protected_custom_attribute_interop_with_keepassxc() {
    let dir = std::env::temp_dir().join(format!("sp-prot-attr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("prot-attr.kdbx");
    let keyfile_path = dir.join("prot-attr.keyfile");
    gen_keyfile(&keyfile_path);

    let config = DatabaseConfig {
        kdf_config: KdfConfig::Argon2id {
            iterations: 3,
            memory: TEST_KDF_MEMORY_BYTES,
            parallelism: 2,
            version: Argon2Version::Version13,
        },
        ..DatabaseConfig::default()
    };
    let db = Database::new(config);
    let mut entry = Entry::default();
    entry.set_title(Some("sp-prot-attr"));
    entry
        .set_additional_attribute_value(
            "sp-api-key",
            Some(keepass_ng::db::Value::protected(
                "sp-protected-attr-value".to_string(),
            )),
        )
        .expect("set protected attr");
    entry
        .set_additional_attribute_value(
            "sp-plain",
            Some(keepass_ng::db::Value::unprotected(
                "sp-plain-value".to_string(),
            )),
        )
        .expect("set plain attr");
    group_add_child(&db.root, rc_refcell_node(entry), 0).expect("add entry");

    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .unwrap();
    let mut out = std::fs::File::create(&db_path).unwrap();
    db.save(&mut out, key).expect("save");

    let db_str = db_path.to_string_lossy().to_string();
    let kf_str = keyfile_path.to_string_lossy().to_string();

    // ---- keepassxc-cli 侧：掩码语义（KeePassXC Show.cpp：仅默认摘要/--all
    // 列表对受保护属性显示 PROTECTED；-a 显式点名即使无 -s 也输出明文——上游
    // 设计如此，故掩码断言用 --all） ----
    let all_masked = kxc(&[
        "show",
        "-q",
        "--all",
        "-k",
        kf_str.as_str(),
        db_str.as_str(),
        "sp-prot-attr",
    ]);
    assert!(
        !all_masked.contains("sp-protected-attr-value"),
        "受保护属性在 --all 列表不得无 -s 泄漏明文: {all_masked}"
    );
    assert!(
        all_masked.contains("sp-api-key: PROTECTED"),
        "受保护属性掩码显示: {all_masked}"
    );
    assert!(
        all_masked.contains("sp-plain-value"),
        "未保护属性明文可见: {all_masked}"
    );

    // ---- keepassxc-cli 侧：-a 显式点名 + -s 解掩码读出（格式层互认） ----
    let revealed = kxc(&[
        "show",
        "-q",
        "-s",
        "-a",
        "sp-api-key",
        "-k",
        kf_str.as_str(),
        db_str.as_str(),
        "sp-prot-attr",
    ]);
    assert!(
        revealed.contains("sp-protected-attr-value"),
        "受保护属性 -s 解掩码读出: {revealed}"
    );

    // ---- sp 侧重开：保护位与值保真（读侧 typed API） ----
    let kf = std::fs::File::open(&keyfile_path).unwrap();
    let key = DatabaseKey::new()
        .with_keyfile(&mut std::io::BufReader::new(kf))
        .unwrap();
    let mut f = std::fs::File::open(&db_path).unwrap();
    let reopened = Database::open(&mut f, key).expect("reopen");
    let (api, plain) = {
        let root = reopened.root.borrow();
        let group = root.downcast_ref::<Group>().expect("root group");
        group
            .entries()
            .iter()
            .find_map(|n| {
                let e = n.borrow();
                e.downcast_ref::<Entry>()
                    .filter(|e| e.get_title() == Some("sp-prot-attr"))
                    .map(|e| {
                        let attrs = e.additional_attributes_typed();
                        let api = attrs
                            .iter()
                            .find(|(k, _)| k == "sp-api-key")
                            .expect("protected attr present")
                            .1
                            .clone();
                        let plain = attrs
                            .iter()
                            .find(|(k, _)| k == "sp-plain")
                            .expect("plain attr present")
                            .1
                            .clone();
                        (api, plain)
                    })
            })
            .expect("entry present")
    };
    assert!(api.is_protected(), "保护位跨 keepassxc 往返保真");
    assert_eq!(api.get(), "sp-protected-attr-value");
    assert!(!plain.is_protected(), "未保护位不受影响");
    assert_eq!(plain.get(), "sp-plain-value");

    let _ = std::fs::remove_dir_all(&dir);
}
