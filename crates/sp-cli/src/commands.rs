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
    /// 受保护自定义字段（S1d-1/DR-017：字段级 ChaCha20 二次加密，KeePassXC 掩码互认）
    pub field_secrets: Vec<(String, String)>,
    pub totp: Option<String>,
    /// 标签分类（SP-F-108；可重复；trim 后空串剔除，校验在 EntryModel::validate）
    pub tags: Vec<String>,
    pub attach: Option<PathBuf>,
}

/// --field 与 --field-secret 合并建 CustomField 列表（后者 protected=true）
fn build_custom_fields(
    fields: &[(String, String)],
    field_secrets: &[(String, String)],
) -> Vec<CustomField> {
    fields
        .iter()
        .map(|(name, value)| CustomField {
            name: name.clone(),
            value: Zeroizing::new(value.clone()),
            protected: false,
        })
        .chain(field_secrets.iter().map(|(name, value)| CustomField {
            name: name.clone(),
            value: Zeroizing::new(value.clone()),
            protected: true,
        }))
        .collect()
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
        field_secrets,
        totp,
        tags,
        attach,
    } = opts;
    let cli_db = cli_db.as_deref();
    let cli_keyfile = cli_keyfile.as_deref();
    // 参数校验前置（2026-10-31 验收反馈）：在解锁（keyfile 加载/KDF）之前完成
    // 无库依赖的校验——坏参数不提示主密码、不进入 KDF 即退出 4。
    EntryModel {
        title: title.clone(),
        username: None,
        password: None,
        url: None,
        notes: None,
        custom_fields: build_custom_fields(&fields, &field_secrets),
        otp: totp.clone(),
        tags: tags.clone(),
    }
    .validate()?;
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
        custom_fields: build_custom_fields(&fields, &field_secrets),
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
    index: Option<usize>,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (db, _factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    let m = sp_kdbx::find_entry_at(&db, &title, index)?;

    if show_totp {
        let code = sp_kdbx::entry_totp_now_at(&db, &title, index)?;
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
                // S1d-1：受保护字段全量展示时掩码（与 GUI 口径一致）；单字段
                // `-a <name>` 为显式请求，仍输出明文
                if c.protected {
                    println!("{}: ●●●●●●●●", c.name);
                } else {
                    println!("{}: {}", c.name, c.value.as_str());
                }
            }
        }
    }
    Ok(())
}

/// 列条目标题（可按标签过滤：精确匹配、大小写不敏感，SP-F-108）。
/// `enumerate=true`（DR-016）：每行输出 `#<同名组内序号>\t<title>`（1-based
/// 文档序，与 item get/delete/edit --index 同一口径）；默认保持纯标题排序输出
/// （脚本兼容）。
pub fn item_list(
    cli_db: Option<&Path>,
    cli_keyfile: Option<&Path>,
    tag: Option<&str>,
    enumerate: bool,
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
    if enumerate {
        use std::collections::HashMap;
        let mut seen: HashMap<String, usize> = HashMap::new();
        for s in sp_kdbx::list_entry_summaries(&db) {
            if let Some(w) = &wanted {
                if !s.tags.iter().any(|t| t.to_lowercase() == *w) {
                    continue;
                }
            }
            let n = seen
                .entry(s.title.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            println!("#{n}\t{}", s.title);
        }
        return Ok(());
    }
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
    index: Option<usize>,
) -> Result<(), SpError> {
    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let c2 = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        let mut c = c2.clone();
        c.baseline_sha256 = cc.baseline_sha256.clone();
        config::save(cc)
    })?;
    sp_kdbx::delete_entry_at(&mut db, &title, index)?;
    save_and_backup(&mut db, &db_path, &factors)?;
    match index {
        Some(n) => println!("Deleted entry '{title}' (#-{n})"),
        None => println!("Deleted entry '{title}'"),
    }
    Ok(())
}

// ---------- item edit（SP-F-102 CLI 补齐，S1c 插入批；DR-016 寻址） ----------

/// item edit 的参数包（patch 语义：未给出的字段保持不变）
pub struct EditOptions {
    pub cli_db: Option<PathBuf>,
    pub cli_keyfile: Option<PathBuf>,
    pub title: String,
    /// 同名组内序号（DR-016；1-based 文档序）
    pub index: Option<usize>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    /// 交互读取新密码（空输入=保持不变）
    pub password: bool,
    pub totp: Option<String>,
    /// 整体替换标签集（空集=清空）
    pub tags: Vec<String>,
    pub fields: Vec<(String, String)>,
    /// 受保护自定义字段 upsert（S1d-1/DR-017；同名键覆盖 --field 的保护位）
    pub field_secrets: Vec<(String, String)>,
    pub remove_fields: Vec<String>,
    pub rename: Option<String>,
}

pub fn item_edit(opts: EditOptions) -> Result<(), SpError> {
    let cli_db = opts.cli_db.as_deref();
    let cli_keyfile = opts.cli_keyfile.as_deref();

    // 参数校验前置（对齐 item create/read/clip 口径）：无库依赖的校验在解锁
    // （keyfile 加载/KDF）之前完成——坏参数不提示主密码、退出 4。
    validate_edit_patch(&opts)?;

    let cfg = config::load()?;
    let db_path = resolve_db(cli_db, &cfg)?;
    let mut c = cfg.clone();
    let (mut db, factors) = open_with_config(&db_path, cli_keyfile, &cfg, |cc| {
        c = cc.clone();
        config::save(cc)
    })?;

    // 解锁后合并 patch（定位仍按原 title；--rename 的效果在合并模型里生效）
    let current = sp_kdbx::find_entry_at(&db, &opts.title, opts.index)?;
    let new_password = if opts.password {
        let pw = read_secret("New entry password (empty to keep current): ")?;
        if pw.is_empty() {
            None
        } else {
            Some(pw)
        }
    } else {
        None
    };
    let merged = apply_edit_patch(current, &opts, new_password)?;
    sp_kdbx::edit_entry_at(&mut db, &opts.title, opts.index, &merged)?;
    save_and_backup(&mut db, &db_path, &factors)?;

    // 变更摘要：只列字段类别与数量，不回显任何值
    let idx_note = opts.index.map(|n| format!(" (#-{n})")).unwrap_or_default();
    match &opts.rename {
        Some(r) => println!(
            "Updated entry '{}'{idx_note} → '{r}': {}",
            opts.title,
            edit_summary(&opts, &merged)
        ),
        None => println!(
            "Updated entry '{}'{idx_note}: {}",
            opts.title,
            edit_summary(&opts, &merged)
        ),
    }
    Ok(())
}

/// 编辑参数的无库依赖校验（退出 4 语义；纯函数，单测见 tests 模块）
fn validate_edit_patch(opts: &EditOptions) -> Result<(), SpError> {
    if opts.index == Some(0) {
        return Err(SpError::Parameter("--index is 1-based".into()));
    }
    if opts.rename.as_deref().map(str::trim) == Some("") {
        return Err(SpError::Parameter("--rename must not be empty".into()));
    }
    if let Some(t) = opts.totp.as_deref().filter(|s| !s.is_empty()) {
        if !t.starts_with("otpauth://") {
            return Err(SpError::Parameter(
                "totp must be an otpauth:// URL (or empty string to clear)".into(),
            ));
        }
    }
    for (name, _) in &opts.fields {
        if name.trim().is_empty() {
            return Err(SpError::Parameter(
                "custom field name must not be empty".into(),
            ));
        }
        if sp_core::RESERVED_FIELD_NAMES.contains(&name.as_str()) {
            return Err(SpError::Parameter(format!(
                "custom field name '{name}' is reserved"
            )));
        }
    }
    for (name, _) in &opts.field_secrets {
        if name.trim().is_empty() {
            return Err(SpError::Parameter(
                "custom field name must not be empty".into(),
            ));
        }
        if sp_core::RESERVED_FIELD_NAMES.contains(&name.as_str()) {
            return Err(SpError::Parameter(format!(
                "custom field name '{name}' is reserved"
            )));
        }
    }
    for name in &opts.remove_fields {
        if name.trim().is_empty() {
            return Err(SpError::Parameter(
                "--remove-field name must not be empty".into(),
            ));
        }
    }
    if opts.username.is_none()
        && opts.url.is_none()
        && opts.notes.is_none()
        && !opts.password
        && opts.totp.is_none()
        && opts.tags.is_empty()
        && opts.fields.is_empty()
        && opts.field_secrets.is_empty()
        && opts.remove_fields.is_empty()
        && opts.rename.is_none()
    {
        return Err(SpError::Parameter(
            "nothing to edit: give at least one of \
             -u/--url/--notes/-p/--totp/--tag/--field/--field-secret/--remove-field/--rename"
                .into(),
        ));
    }
    Ok(())
}

/// 合并补丁到现模型（纯函数；new_password=None 表示保持现密码）。
/// patch 语义：未给出的字段保持不变；-u/--url/--notes/--totp 空串=清除；
/// --tag 整体替换（trim 后空串剔除，全空=清空）；--field upsert（不动既有的
/// 保护位）；--field-secret upsert（置保护位，同名覆盖 --field）；
/// --remove-field 按名删除；--rename 改写标题。
fn apply_edit_patch(
    mut current: EntryModel,
    opts: &EditOptions,
    new_password: Option<Zeroizing<String>>,
) -> Result<EntryModel, SpError> {
    if let Some(u) = &opts.username {
        current.username = if u.is_empty() { None } else { Some(u.clone()) };
    }
    if let Some(v) = &opts.url {
        current.url = if v.is_empty() { None } else { Some(v.clone()) };
    }
    if let Some(v) = &opts.notes {
        current.notes = if v.is_empty() { None } else { Some(v.clone()) };
    }
    if let Some(pw) = new_password {
        current.password = Some(pw);
    }
    if let Some(t) = &opts.totp {
        current.otp = if t.is_empty() { None } else { Some(t.clone()) };
    }
    if !opts.tags.is_empty() {
        current.tags = opts
            .tags
            .iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
    }
    for (name, value) in &opts.fields {
        if let Some(f) = current.custom_fields.iter_mut().find(|f| &f.name == name) {
            f.value = Zeroizing::new(value.clone());
        } else {
            current.custom_fields.push(CustomField {
                name: name.clone(),
                value: Zeroizing::new(value.clone()),
                protected: false,
            });
        }
    }
    // --field-secret 后应用：同名键覆盖 --field 的值与保护位（S1d-1/DR-017）
    for (name, value) in &opts.field_secrets {
        if let Some(f) = current.custom_fields.iter_mut().find(|f| &f.name == name) {
            f.value = Zeroizing::new(value.clone());
            f.protected = true;
        } else {
            current.custom_fields.push(CustomField {
                name: name.clone(),
                value: Zeroizing::new(value.clone()),
                protected: true,
            });
        }
    }
    if !opts.remove_fields.is_empty() {
        current
            .custom_fields
            .retain(|f| !opts.remove_fields.contains(&f.name));
    }
    if let Some(r) = &opts.rename {
        current.title = r.clone();
    }
    current.validate()?;
    Ok(current)
}

/// 变更摘要（只列字段类别与数量，不回显任何值）
fn edit_summary(opts: &EditOptions, merged: &EntryModel) -> String {
    let mut parts: Vec<String> = Vec::new();
    if opts.username.is_some() {
        parts.push("username".into());
    }
    if opts.password {
        parts.push("password".into());
    }
    if opts.url.is_some() {
        parts.push("url".into());
    }
    if opts.notes.is_some() {
        parts.push("notes".into());
    }
    if opts.totp.is_some() {
        parts.push("totp".into());
    }
    if !opts.tags.is_empty() {
        parts.push(format!("tags({})", merged.tags.len()));
    }
    if !opts.fields.is_empty() {
        parts.push(format!("custom_fields(+{})", opts.fields.len()));
    }
    if !opts.field_secrets.is_empty() {
        parts.push(format!(
            "custom_fields_secret(+{})",
            opts.field_secrets.len()
        ));
    }
    if !opts.remove_fields.is_empty() {
        parts.push(format!("custom_fields(-{})", opts.remove_fields.len()));
    }
    parts.join(", ")
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
    index: Option<usize>,
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
    let m = sp_kdbx::find_entry_at(&db, &title, index)?;
    let v = match field.as_str() {
        "Title" => Some(m.title),
        "UserName" => m.username,
        "Password" => m.password.as_ref().map(|p| p.as_str().to_string()),
        "URL" => m.url,
        "Notes" => m.notes,
        "totp" | "TOTP" => {
            let code = sp_kdbx::entry_totp_now_at(&db, &title, index)?;
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
    index: Option<usize>,
) -> Result<(), SpError> {
    // URI 格式校验前置：坏参数在解锁（KDF）之前即报参数错误（2026-10-31 验收反馈）
    parse_uri_public(&uri)?;
    if index == Some(0) {
        return Err(SpError::Parameter("--index is 1-based".into()));
    }
    let value = read_value(cli_db, cli_keyfile, &uri, index)?;
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
    index: Option<usize>,
) -> Result<(), SpError> {
    // URI 格式校验前置：坏参数在解锁（KDF）之前即报参数错误（2026-10-31 验收反馈）
    parse_uri_public(&uri)?;
    if index == Some(0) {
        return Err(SpError::Parameter("--index is 1-based".into()));
    }
    let value = read_value(cli_db, cli_keyfile, &uri, index)?;
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

#[cfg(test)]
mod item_edit_tests {
    use super::*;

    fn opts() -> EditOptions {
        EditOptions {
            cli_db: None,
            cli_keyfile: None,
            title: "svc".into(),
            index: None,
            username: None,
            url: None,
            notes: None,
            password: false,
            totp: None,
            tags: vec![],
            fields: vec![],
            field_secrets: vec![],
            remove_fields: vec![],
            rename: None,
        }
    }

    fn base_model() -> EntryModel {
        EntryModel {
            title: "svc".into(),
            username: Some("u1".into()),
            password: Some(Zeroizing::new("sp-test-only-pw".into())),
            url: Some("https://old.local".into()),
            notes: Some("n".into()),
            custom_fields: vec![CustomField {
                name: "env".into(),
                value: Zeroizing::new("dev".into()),
                protected: false,
            }],
            otp: None,
            tags: vec!["work".into()],
        }
    }

    #[test]
    fn validate_rejects_nothing_to_edit_and_bad_params() {
        let o = opts();
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("nothing to edit"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }

        let mut o = opts();
        o.index = Some(0);
        o.username = Some("x".into());
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("1-based"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }

        let mut o = opts();
        o.rename = Some("  ".into());
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("rename"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }

        let mut o = opts();
        o.totp = Some("not-otpauth".into());
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("otpauth"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }

        let mut o = opts();
        o.fields = vec![("Title".into(), "x".into())];
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("reserved"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }

        let mut o = opts();
        o.fields = vec![("env".into(), "prod".into())];
        assert!(validate_edit_patch(&o).is_ok());
    }

    #[test]
    fn apply_patch_semantics() {
        let mut o = opts();
        o.username = Some("".into()); // 空串清除
        o.url = Some("https://new.local".into());
        o.totp = Some("otpauth://totp/x?secret=JBSWY3DPEHPK3PXP".into());
        o.tags = vec![" infra ".into(), "".into()]; // trim+剔空 → 整体替换
        o.fields = vec![
            ("env".into(), "prod".into()),
            ("region".into(), "cn".into()),
        ];
        o.remove_fields = vec!["gone".into()]; // 不存在的名字，无副作用
        o.rename = Some("svc-renamed".into());
        let m = apply_edit_patch(base_model(), &o, None).expect("apply");
        assert_eq!(m.username, None, "空串清除 username");
        assert_eq!(m.url.as_deref(), Some("https://new.local"));
        assert!(m.otp.is_some());
        assert_eq!(m.tags, vec!["infra".to_string()]);
        assert_eq!(m.custom_fields.len(), 2, "env upsert + region 新增");
        let env = m
            .custom_fields
            .iter()
            .find(|f| f.name == "env")
            .expect("env field");
        assert_eq!(env.value.as_str(), "prod");
        assert_eq!(m.title, "svc-renamed");
        // password 补丁 None=保持现值
        assert_eq!(
            m.password.as_ref().map(|p| p.as_str().to_string()),
            Some("sp-test-only-pw".to_string())
        );
    }

    #[test]
    fn apply_patch_password_replacement_only_when_given() {
        let o = opts();
        let m = apply_edit_patch(base_model(), &o, None).expect("apply None");
        assert_eq!(
            m.password.as_ref().map(|p| p.as_str().to_string()),
            Some("sp-test-only-pw".to_string())
        );
        let m = apply_edit_patch(
            base_model(),
            &o,
            Some(Zeroizing::new("sp-test-only-new".into())),
        )
        .expect("apply Some");
        assert_eq!(
            m.password.as_ref().map(|p| p.as_str().to_string()),
            Some("sp-test-only-new".to_string())
        );
    }

    #[test]
    fn apply_patch_field_secret_overrides_field_protection() {
        // S1d-1：--field 与 --field-secret 同名相遇，后者覆盖值与保护位；
        // --field upsert 既有受保护字段时不动保护位
        let mut o = opts();
        o.fields = vec![
            ("env".into(), "prod".into()),    // 既有未保护 upsert
            ("api_key".into(), "k-2".into()), // 既有受保护 upsert（base_model 里无）
        ];
        o.field_secrets = vec![
            ("api_key".into(), "k-secret".into()), // 覆盖上面 --field 的 api_key
            ("token".into(), "t-1".into()),        // 新增受保护
        ];
        let m = apply_edit_patch(base_model(), &o, None).expect("apply");
        let env = m
            .custom_fields
            .iter()
            .find(|f| f.name == "env")
            .expect("env");
        assert_eq!(env.value.as_str(), "prod");
        assert!(!env.protected, "--field 不改既有保护位");
        let api = m
            .custom_fields
            .iter()
            .find(|f| f.name == "api_key")
            .expect("api_key");
        assert_eq!(api.value.as_str(), "k-secret");
        assert!(api.protected, "--field-secret 同名覆盖 --field");
        let token = m
            .custom_fields
            .iter()
            .find(|f| f.name == "token")
            .expect("token");
        assert!(token.protected);
        assert_eq!(token.value.as_str(), "t-1");
    }

    #[test]
    fn validate_rejects_field_secret_reserved_name() {
        let mut o = opts();
        o.field_secrets = vec![("Title".into(), "x".into())];
        match validate_edit_patch(&o) {
            Err(SpError::Parameter(m)) => assert!(m.contains("reserved"), "{m}"),
            other => panic!("expected Parameter, got {other:?}"),
        }
        let mut o = opts();
        o.field_secrets = vec![("token".into(), "t".into())];
        assert!(validate_edit_patch(&o).is_ok());
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
