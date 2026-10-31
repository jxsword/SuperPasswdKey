//! 命令实现（H3）：init / item / read / clip。

use std::io::Write as _;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use sp_kdbx::engine::db::Database;

use sp_core::{calibration, CredentialFactors, CustomField, EntryModel, SpError};

use crate::config::{self, Config};
use crate::integrity;

// ---------- 公共 ----------

fn resolve_db(cli_db: Option<&Path>, cfg: &Config) -> Result<PathBuf, SpError> {
    cli_db
        .map(PathBuf::from)
        .or_else(|| cfg.default_db.clone())
        .ok_or_else(|| {
            SpError::Config(
                "no database configured; run `sp init` first or pass --db <path>".into(),
            )
        })
}

/// 读取秘密输入（SP-F-105）：TTY → rpassword 隐藏回显；非 TTY（脚本/CI）→ stdin 行读取。
/// Windows 实测：rpassword::read_password 只读控制台 CONIN$，管道 stdin 会永久阻塞。
/// 提示写 **stderr**：stdout 保持纯净数据通道（`sp read` 管道、`attachment get` 原始
/// 字节输出不被提示文本污染——2026-10-29 实测修正）。
pub(crate) fn read_secret(prompt: &str) -> Result<Zeroizing<String>, SpError> {
    use std::io::IsTerminal;
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    if std::io::stdin().is_terminal() {
        let p =
            rpassword::read_password().map_err(|e| SpError::Io(format!("read password: {e}")))?;
        Ok(Zeroizing::new(p))
    } else {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| SpError::Io(format!("read stdin: {e}")))?;
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        Ok(Zeroizing::new(line))
    }
}

/// 交互收集凭据并解锁（SP-F-105：v0.1 每命令独立解锁，无跨进程会话）
fn unlock(
    db: &Path,
    cli_keyfile: Option<&Path>,
    cfg: &Config,
) -> Result<CredentialFactors, SpError> {
    let keyfile = cli_keyfile
        .map(PathBuf::from)
        .or_else(|| cfg.keyfile.clone());
    let password = read_secret(&format!("Enter password to unlock {}: ", db.display()))?;
    Ok(CredentialFactors {
        password,
        keyfile_path: keyfile,
    })
}

fn open_with_config(
    db: &Path,
    cli_keyfile: Option<&Path>,
    cfg: &Config,
    mut cfg_update: impl FnMut(&Config) -> Result<(), SpError>,
) -> Result<(Database, CredentialFactors), SpError> {
    let factors = unlock(db, cli_keyfile, cfg)?;
    let database = match sp_kdbx::open_db(db, &factors) {
        Ok(d) => d,
        Err(e) => {
            // SP-N-311：打开失败且哈希≠基线 → 外部修改提示
            if let Some(_current) = integrity::detect_external_change(db, &cfg.baseline_sha256) {
                eprintln!(
                    "WARNING: database hash differs from last known baseline; \
                     the file may have been externally modified or corrupted."
                );
            }
            return Err(e);
        }
    };
    // 打开成功 → 静默更新基线（多设备同步为正常场景，防误报——复审 R2-3）
    if let Ok(h) = integrity::sha256_file(db) {
        if cfg.baseline_sha256.as_deref() != Some(h.as_str()) {
            let mut c = cfg.clone();
            c.baseline_sha256 = Some(h);
            cfg_update(&c)?;
        }
    }
    Ok((database, factors))
}

fn save_and_backup(
    db: &mut Database,
    db_path: &Path,
    factors: &CredentialFactors,
) -> Result<(), SpError> {
    sp_kdbx::save_atomic(db, db_path, factors)?;
    integrity::rotate_backup(db_path, 10) // SP-N-312：滚动 10 份
}

// ---------- init ----------

pub fn init(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    kdf_target_ms: u64,
) -> Result<(), SpError> {
    let mut cfg = config::load()?;
    let db = resolve_db(cli_db, &Config::default())?;
    if db.exists() {
        return Err(SpError::Parameter(format!(
            "database already exists at {} (use --db for another location)",
            db.display()
        )));
    }
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SpError::Io(format!("create db dir: {e}")))?;
    }

    let pw1 = read_secret("Choose a master password: ")?;
    let pw2 = read_secret("Confirm master password: ")?;
    if pw1.as_str() != pw2.as_str() {
        return Err(SpError::Parameter("passwords do not match".into()));
    }

    // keyfile：生成官方 XML v1.0（DR-005）
    let keyfile = match cli_keyfile {
        Some(k) => PathBuf::from(k),
        None => {
            let k = db.with_extension("keyfile");
            println!(
                "Generating keyfile (XML v1.0) at {} — keep it on a SEPARATE medium!",
                k.display()
            );
            k
        }
    };
    if !keyfile.exists() {
        sp_kdbx::generate_keyfile(&keyfile)?;
    }

    // KDF 校准（SP-N-302）：目标毫秒
    println!("Calibrating Argon2id to ~{kdf_target_ms}ms (this takes a moment)...");
    let params = calibration::calibrate(kdf_target_ms)?;
    println!(
        "KDF: Argon2id m={}MiB t={} p={}",
        params.memory_kib / 1024,
        params.iterations,
        params.parallelism
    );

    let factors = CredentialFactors {
        password: pw1,
        keyfile_path: Some(keyfile.clone()),
    };
    sp_kdbx::create_db(&db, &factors, &params)?;

    // 写配置 + 基线
    cfg.default_db = Some(db.clone());
    cfg.keyfile = Some(keyfile);
    cfg.baseline_sha256 = Some(integrity::sha256_file(&db)?);
    config::save(&cfg)?;
    println!("Database created at {}", db.display());
    println!("REMINDER: back up the keyfile to a separate medium (USB/another device).");
    Ok(())
}

// ---------- item ----------

/// item create 的参数包（避免长参数列表）
pub struct CreateOptions {
    pub cli_db: Option<PathBuf>,
    pub cli_keyfile: Option<PathBuf>,
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub fields: Vec<(String, String)>,
    pub totp: Option<String>,
    /// 标签分类（SP-F-108；可重复；trim 后空串剔除，校验在 EntryModel::validate）
    pub tags: Vec<String>,
    pub attach: Option<PathBuf>,
}

pub fn item_create(opts: CreateOptions) -> Result<(), SpError> {
    let CreateOptions {
        cli_db,
        cli_keyfile,
        title,
        username,
        url,
        notes,
        fields,
        totp,
        tags,
        attach,
    } = opts;
    let cli_db = cli_db.as_deref();
    let cli_keyfile = cli_keyfile.as_deref();
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let mut c = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        c = cc.clone();
        config::save(cc)
    })?;
    // 解锁后再收集条目密码（顺序修正：解锁优先）
    let pw = read_secret("Entry password (empty to skip): ")?;

    let attach_name = attach
        .as_ref()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let model = EntryModel {
        title: title.clone(),
        username,
        password: if pw.is_empty() { None } else { Some(pw) },
        url,
        notes,
        custom_fields: fields
            .into_iter()
            .map(|(name, value)| CustomField {
                name,
                value: Zeroizing::new(value),
                protected: false, // v0.1 限制（DR-005 记录）
            })
            .collect(),
        otp: totp,
        tags: tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
    };
    sp_kdbx::add_entry(&mut db, &model)?;
    // 附件（SP-F-104）
    if let (Some(path), Some(name)) = (&attach, &attach_name) {
        let data = std::fs::read(path).map_err(|e| SpError::Io(format!("read attachment: {e}")))?;
        sp_kdbx::add_attachment(&mut db, &title, name, &data)?;
    }
    save_and_backup(&mut db, &db_path, &factors)?;
    println!("Created entry '{title}'");
    Ok(())
}

pub fn item_get(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    title: String,
    field: Option<String>,
    show_totp: bool,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    let m = sp_kdbx::find_entry(&db, &title)?;

    if show_totp {
        let code = sp_kdbx::entry_totp_now(&db, &title)?;
        println!("{code}");
        return Ok(());
    }
    match field {
        Some(f) => {
            let v = m
                .custom_fields
                .iter()
                .find(|c| c.name == f)
                .map(|c| c.value.as_str().to_string())
                .or_else(|| match f.as_str() {
                    "Title" => Some(m.title.clone()),
                    "UserName" => m.username.clone(),
                    "Password" => m.password.as_ref().map(|p| p.as_str().to_string()),
                    "URL" => m.url.clone(),
                    "Notes" => m.notes.clone(),
                    _ => None,
                })
                .ok_or_else(|| SpError::NotFound(format!("{title}/{f}")))?;
            println!("{v}");
        }
        None => {
            println!("Title: {}", m.title);
            println!("UserName: {}", m.username.unwrap_or_default());
            println!(
                "Password: {}",
                m.password.as_ref().map(|p| p.as_str()).unwrap_or("")
            );
            println!("URL: {}", m.url.unwrap_or_default());
            if !m.tags.is_empty() {
                println!("Tags: {}", m.tags.join(", "));
            }
            if !m.notes.as_deref().unwrap_or("").is_empty() {
                println!("Notes: {}", m.notes.as_deref().unwrap_or(""));
            }
            for c in &m.custom_fields {
                println!("{}: {}", c.name, c.value.as_str());
            }
        }
    }
    Ok(())
}

/// 列条目标题（可按标签过滤：精确匹配、大小写不敏感；SP-F-108）
pub fn item_list(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    tag: Option<&str>,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    let wanted = tag.map(|t| t.to_lowercase());
    let mut titles: Vec<String> = sp_kdbx::list_entry_summaries(&db)
        .into_iter()
        .filter(|s| match &wanted {
            None => true,
            Some(w) => s.tags.iter().any(|t| t.to_lowercase() == *w),
        })
        .map(|s| s.title)
        .collect();
    titles.sort();
    for t in titles {
        println!("{t}");
    }
    Ok(())
}

pub fn item_delete(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    title: String,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    sp_kdbx::delete_entry(&mut db, &title)?;
    save_and_backup(&mut db, &db_path, &factors)?;
    println!("Deleted entry '{title}'");
    Ok(())
}

// ---------- 附件（SP-F-104 补全：add/list/get） ----------

/// 给已有条目添加/替换附件（同名覆盖；SP-F-104）
pub fn attachment_add(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    title: &str,
    file: &Path,
    name: Option<&str>,
) -> Result<(), SpError> {
    let data = std::fs::read(file)
        .map_err(|e| SpError::Io(format!("read attachment '{}': {e}", file.display())))?;
    let name = match name {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| {
                SpError::Parameter("cannot derive attachment name from path; use --name".into())
            })?,
    };
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    sp_kdbx::add_attachment(&mut db, title, &name, &data)?;
    save_and_backup(&mut db, &db_path, &factors)?;
    println!(
        "Attachment '{name}' ({} bytes) saved to entry '{title}' (same name replaces).",
        data.len()
    );
    Ok(())
}

/// 列出条目的全部附件名（排序；无附件时不输出）
pub fn attachment_list(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    title: &str,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    for n in sp_kdbx::list_attachments(&db, title)? {
        println!("{n}");
    }
    Ok(())
}

/// 取回附件：--out 写文件（ACL 限当前用户），否则原始字节写 stdout
pub fn attachment_get(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    title: &str,
    name: &str,
    out: Option<PathBuf>,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    let data = sp_kdbx::get_attachment(&db, title, name)?;
    match out {
        Some(path) => {
            std::fs::write(&path, &data)
                .map_err(|e| SpError::Io(format!("write '{}': {e}", path.display())))?;
            restrict_perms_public(&path)?;
            eprintln!(
                "Attachment '{name}' ({} bytes) written to {}",
                data.len(),
                path.display()
            );
        }
        None => {
            use std::io::Write as _;
            std::io::stdout()
                .write_all(&data)
                .map_err(|e| SpError::Io(format!("write stdout: {e}")))?;
            eprintln!(
                "Attachment '{name}' ({} bytes) written to stdout as raw bytes.",
                data.len()
            );
        }
    }
    Ok(())
}

// ---------- read / clip ----------

/// 解析引用 URI（SP-F-202）："sp://<title>/<field>" 或 "<title>/<field>"；段不可省略
pub(crate) fn parse_uri_public(uri: &str) -> Result<(String, String), SpError> {
    parse_uri(uri)
}

fn parse_uri(uri: &str) -> Result<(String, String), SpError> {
    let body = uri.strip_prefix("sp://").unwrap_or(uri);
    let (title, field) = body
        .split_once('/')
        .ok_or_else(|| SpError::Parameter(format!("uri '{uri}' must be <title>/<field>")))?;
    if title.is_empty() || field.is_empty() {
        return Err(SpError::Parameter(format!(
            "uri '{uri}' must be <title>/<field>"
        )));
    }
    Ok((title.to_string(), field.to_string()))
}

fn read_value(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    uri: &str,
) -> Result<Zeroizing<String>, SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    let (title, field) = parse_uri(uri)?;
    let m = sp_kdbx::find_entry(&db, &title)?;
    let v = match field.as_str() {
        "Title" => Some(m.title),
        "UserName" => m.username,
        "Password" => m.password.as_ref().map(|p| p.as_str().to_string()),
        "URL" => m.url,
        "Notes" => m.notes,
        "totp" | "TOTP" => {
            let code = sp_kdbx::entry_totp_now(&db, &title)?;
            return Ok(Zeroizing::new(code));
        }
        f => m
            .custom_fields
            .iter()
            .find(|c| c.name == f)
            .map(|c| c.value.as_str().to_string()),
    }
    .ok_or_else(|| SpError::NotFound(format!("{title}/{field}")))?;
    Ok(Zeroizing::new(v))
}

pub fn read(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    uri: String,
    no_newline: bool,
    out_file: Option<PathBuf>,
) -> Result<(), SpError> {
    let value = read_value(cli_db, cli_keyfile, &uri)?;
    match out_file {
        Some(path) => {
            std::fs::write(&path, value.as_bytes())
                .map_err(|e| SpError::Io(format!("write out-file: {e}")))?;
            restrict_perms(&path)?;
        }
        None => {
            if no_newline {
                print!("{}", value.as_str());
            } else {
                println!("{}", value.as_str());
            }
        }
    }
    Ok(())
}

/// --out-file 权限：Windows ACL 限当前用户（复审 R2 修正：无 POSIX 0600 语义）
pub(crate) fn restrict_perms_public(path: &Path) -> Result<(), SpError> {
    // 单源下沉（2026-10-31）：逻辑移至 sp-core::fsperms，桌面 GUI 附件下载复用同源
    sp_core::fsperms::restrict(path)
}

fn restrict_perms(path: &Path) -> Result<(), SpError> {
    restrict_perms_public(path)
}

pub fn clip(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    uri: String,
    timeout_secs: Option<u64>,
) -> Result<(), SpError> {
    let value = read_value(cli_db, cli_keyfile, &uri)?;
    let cfg = config::load()?;
    let timeout = timeout_secs.unwrap_or(cfg.clip_timeout_secs);
    let mut ctx = arboard::Clipboard::new()
        .map_err(|e| SpError::Clipboard(format!("init clipboard: {e}")))?;
    ctx.set_text(value.as_str().to_string())
        .map_err(|e| SpError::Clipboard(format!("set clipboard: {e}")))?;
    println!(
        "Copied to clipboard; clearing in {timeout}s (Win+V history is NOT controlled by sp)."
    );
    std::thread::sleep(std::time::Duration::from_secs(timeout));
    ctx.set_text(String::new())
        .map_err(|e| SpError::Clipboard(format!("clear clipboard: {e}")))?;
    Ok(())
}

// ---------- F9 导入（SP-F-401~403） ----------

/// 导入参数包
pub struct ImportOptions {
    pub cli_db: Option<PathBuf>,
    pub cli_keyfile: Option<PathBuf>,
    pub kind: String,
    pub path: Option<PathBuf>,
    pub dry_run: bool,
}

pub fn import(opts: ImportOptions) -> Result<(), SpError> {
    let ImportOptions {
        cli_db,
        cli_keyfile,
        kind,
        path,
        dry_run,
    } = opts;
    let cli_db = cli_db.as_deref();
    let cli_keyfile = cli_keyfile.as_deref();
    let findings = match kind.as_str() {
        "dotenv" => crate::importer::parse_dotenv(
            path.as_deref()
                .ok_or_else(|| SpError::Parameter("dotenv requires <path>".into()))?,
        )?,
        "csv" => crate::importer::parse_csv(
            path.as_deref()
                .ok_or_else(|| SpError::Parameter("csv requires <path>".into()))?,
        )?,
        "bw-json" => crate::importer::parse_bw_json(
            path.as_deref()
                .ok_or_else(|| SpError::Parameter("bw-json requires <path>".into()))?,
        )?,
        "env" => crate::importer::parse_env_snapshot(),
        "files" => crate::importer::parse_files(
            path.as_deref()
                .ok_or_else(|| SpError::Parameter("files requires <path>".into()))?,
        )?,
        other => return Err(SpError::Parameter(format!("unknown import kind '{other}'"))),
    };
    let report = crate::importer::build_candidates(findings);
    println!(
        "Scan: {} findings -> {} unique candidates (L1 dup {} / L2 merged {} / L3 conflicts {})",
        report.total, report.unique, report.l1_duplicates, report.l2_merged, report.l3_conflicts
    );
    for m in &report.manual_review {
        println!("REVIEW {m}");
    }
    if dry_run {
        println!("Dry run: nothing imported.");
        return Ok(());
    }
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    for m in &report.candidates {
        sp_kdbx::add_entry(&mut db, m)?;
    }
    save_and_backup(&mut db, &db_path, &factors)?;
    println!("Imported {} entries.", report.candidates.len());
    Ok(())
}

// ---------- F10 导出（SP-F-501~503） ----------

/// 导出参数包
pub struct ExportOptions {
    pub cli_db: Option<PathBuf>,
    pub cli_keyfile: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub plaintext: bool,
    pub stdout: bool,
}

pub fn export_all(opts: ExportOptions) -> Result<(), SpError> {
    let ExportOptions {
        cli_db,
        cli_keyfile,
        out,
        plaintext,
        stdout,
    } = opts;
    let cli_db = cli_db.as_deref();
    let cli_keyfile = cli_keyfile.as_deref();
    let out = out.as_deref();
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    if plaintext {
        // 三重防护之二：重输主密码确认
        crate::export::confirm_master_password(&factors)?;
        if !stdout {
            eprintln!("NOTE: plaintext export goes to stdout; no temp file is written.");
        }
        crate::export::export_plaintext_stdout(&db)
    } else {
        let out =
            out.ok_or_else(|| SpError::Parameter("encrypted export requires --out <file>".into()))?;
        crate::export::export_encrypted(&db, out)
    }
}

#[cfg(all(test, unix))]
mod posix_perms_tests {
    use super::*;

    /// POSIX 0600 分支实测（design_docs/03 §5：--out-file / inject 产物权限）
    #[test]
    fn restrict_perms_sets_mode_0600() {
        let path = std::env::temp_dir().join(format!(
            "sp-perms-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"secret").unwrap();
        restrict_perms_public(&path).expect("restrict perms");
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "out-file must be 0600 on POSIX");
        let _ = std::fs::remove_file(&path);
    }
}
