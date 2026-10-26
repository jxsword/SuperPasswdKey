//! `sp run --stdin` e2e（POSIX 专属，design_docs/04 §4.1）：
//! 机密以 NUL 分隔记录送达子进程 stdin、不出现在子进程环境、
//! 输出经遮蔽过滤器转发（SP-F-301/303）。

#![cfg(unix)]

use std::process::{Command, Stdio};
use zeroize::Zeroizing;

const PASSWORD: &str = "e2e-test-only";
const SECRET: &str = "sekret-t0ken-value";

#[test]
fn run_stdin_delivers_records_and_masks_output() {
    let dir = std::env::temp_dir().join(format!("sp-stdin-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("e2e.kdbx");

    let factors = sp_core::CredentialFactors::new(Zeroizing::new(PASSWORD.into()), None);
    sp_kdbx::create_db(&db_path, &factors, &sp_core::KdfParams::default()).unwrap();
    let mut db = sp_kdbx::open_db(&db_path, &factors).unwrap();
    sp_kdbx::add_entry(
        &mut db,
        &sp_core::EntryModel {
            title: "svc".into(),
            password: Some(Zeroizing::new(SECRET.into())),
            ..Default::default()
        },
    )
    .unwrap();
    sp_kdbx::save_atomic(&db, &db_path, &factors).unwrap();

    let env_file = dir.join("app.env");
    std::fs::write(&env_file, "TOKEN=sp://svc/Password\n").unwrap();

    // 子进程：逐条读 NUL 分隔记录并回显；同时证明机密不在其环境中
    let child_script = "while IFS= read -r -d \"\" rec; do printf \"%s\\n\" \"$rec\"; done; \
                        if [ -n \"$TOKEN\" ]; then echo ENV_LEAK; fi";
    let mut child = Command::new(env!("CARGO_BIN_EXE_sp"))
        .args([
            "--db",
            db_path.to_str().unwrap(),
            "run",
            "--stdin",
            "--env-file",
            env_file.to_str().unwrap(),
            "--",
            "bash",
            "-c",
            child_script,
        ])
        .env("HOME", &dir) // 隔离 ~/.config/sp/config.toml（基线哈希写入）
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        // 非 TTY 路径：父进程主密码走 stdin 行读取（commands::read_secret）
        use std::io::Write as _;
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(PASSWORD.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "sp run --stdin failed: status={:?} stderr={stderr}",
        out.status
    );
    assert!(
        stdout.contains("TOKEN=<redacted>"),
        "record must reach child stdin and secret must be masked: stdout={stdout} stderr={stderr}"
    );
    assert!(
        !stdout.contains("ENV_LEAK"),
        "secret must not be injected into child env: stdout={stdout}"
    );
    assert!(!stdout.contains(SECRET), "secret leaked in clear: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}
