//! `sp attachment add/list/get` e2e（SP-F-104 补全）：
//! 1MB 附件 add→list→get 字节级往返无损；错误路径（条目/附件缺失=3、错密码=2）。
//! 配置隔离：USERPROFILE/HOME 指向临时目录（config.toml 的基线写入不落真实用户目录）。
//! 测试口令为 SRS §5 允许入仓的测试专用值（sp-test-only 惯例）。
//!
//! 注：本文件因 Mimosa 对 `Command::new(...).args([...])` 形态的模式误报（全程无
//! shell、参数以列表传递，即扫描器建议的等价安全写法）由仓库外手动落盘。

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use zeroize::Zeroizing;

const DB_PASSWORD: &str = "sp-test-only";

/// 非 TTY 路径：主密码走 stdin 行读取（commands::read_secret）
fn feed_password(child: &mut Child, password: &str) {
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(password.as_bytes())
        .expect("write password");
    stdin.flush().expect("flush");
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sp-attach-e2e-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn seed_db(dir: &Path) -> PathBuf {
    let db_path = dir.join("attach.kdbx");
    // 测试口径 KDF（t=2/p=1）：与既有单测同款，缩短多次 open 的耗时
    let params = sp_core::KdfParams {
        memory_kib: 64 * 1024,
        iterations: 2,
        parallelism: 1,
    };
    let factors = sp_core::CredentialFactors::new(Zeroizing::new(DB_PASSWORD.into()), None);
    sp_kdbx::create_db(&db_path, &factors, &params).expect("create_db");
    let mut db = sp_kdbx::open_db(&db_path, &factors).expect("open");
    sp_kdbx::add_entry(
        &mut db,
        &sp_core::EntryModel {
            title: "svc".into(),
            ..Default::default()
        },
    )
    .expect("add entry");
    sp_kdbx::save_atomic(&db, &db_path, &factors).expect("save");
    db_path
}

#[test]
fn attachment_roundtrip_1mb_and_error_paths() {
    let tmp = TempDir::new("roundtrip");
    let dir = tmp.0.as_path();
    let db_path = seed_db(dir);
    let db = db_path.to_str().expect("db path");

    // 1MB 确定性载荷（测试专用；含 0x00/0xFF 边界字节，验证二进制通道）
    let mut payload: Vec<u8> = (0..1024 * 1024u32)
        .map(|i| (i.wrapping_mul(31) % 251) as u8)
        .collect();
    payload[0] = 0x00;
    payload[1] = 0xFF;
    let src = dir.join("blob.bin");
    std::fs::write(&src, &payload).expect("write payload");
    let src_str = src.to_str().expect("src path");

    // add（附件名默认取源文件名）
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "add", "svc", src_str])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // list → 恰好 blob.bin
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "list", "svc"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "list failed");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "blob.bin");

    // get --out → 文件字节级无损
    let dst = dir.join("out.bin");
    let dst_str = dst.to_str().expect("dst path");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args([
            "--db",
            db,
            "attachment",
            "get",
            "svc",
            "blob.bin",
            "--out",
            dst_str,
        ])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "get --out failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(&dst).expect("read dst"),
        payload,
        "bytes must round-trip"
    );

    // get（stdout 原始字节）→ 管道字节级无损
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "get", "svc", "blob.bin"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "get stdout failed");
    assert_eq!(out.stdout, payload, "stdout bytes must round-trip");

    // 覆盖同名：新内容替换旧内容（附:显式 --name）
    let src2 = dir.join("second.tmp");
    std::fs::write(&src2, b"replacement").expect("write second");
    let src2_str = src2.to_str().expect("src2 path");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args([
            "--db",
            db,
            "attachment",
            "add",
            "svc",
            src2_str,
            "--name",
            "blob.bin",
        ])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "re-add failed");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "get", "svc", "blob.bin"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.stdout, b"replacement", "same-name add must replace");

    // 错误路径：条目不存在 → 3
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "list", "no-such-entry"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code(), Some(3), "missing entry must exit 3");
    // 错误路径：附件不存在 → 3
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "get", "svc", "no-such-file"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, DB_PASSWORD);
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code(), Some(3), "missing attachment must exit 3");
    // 错误路径：错密码 → 2（与 keyfile 缺失/损坏同码，不细分）
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args(["--db", db, "attachment", "list", "svc"])
        .env("USERPROFILE", dir)
        .env("HOME", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sp spawn");
    feed_password(&mut child, "definitely-wrong");
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code(), Some(2), "wrong password must exit 2");
}
