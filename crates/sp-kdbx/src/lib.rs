//! sp-kdbx：KDBX4 适配层（keepass_ng 封装，DR-005）。
//!
//! 设计见 design_docs/02：
//! - 库内加密由 KDBX4 格式承担；本层负责映射、原子保存、keyfile 生成（XML v1.0）。
//! - 回退路径：KeepassRsAdapter（DR-005 备选），不改上层。
//! - 已知限制（DR-005 记录）：keepass_ng 公共 API 仅支持未保护自定义字段
//!   （set_additional_attribute 拒绝保留名且无受保护变体），CustomField.protected
//!   在 v0.1 存储为未保护；主密码字段仍为受保护存储。

pub use keepass_ng as engine;

use std::io::Write as _;
use std::path::Path;

use base64::Engine as _;
use keepass_ng::db::{
    group_add_child, rc_refcell_node, Attachment, Database, Entry, Group, Node, Value,
};
use keepass_ng::{Argon2Version, DatabaseConfig, DatabaseKey, KdfConfig};
use zeroize::Zeroizing;

use sp_core::{CredentialFactors, EntryModel, KdfParams, SpError};

pub const ADAPTER_NAME: &str = "keepass_ng";

// ---------- keyfile（DR-005：必须官方 XML v1.0，legacy raw 格式语义分歧） ----------

/// 生成官方 XML KeyFile v1.0 密钥文件（32B 熵，Base64）。
pub fn generate_keyfile(path: &Path) -> Result<(), SpError> {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).map_err(|e| SpError::Io(format!("os rng: {e}")))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>1.00</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data>{b64}</Data>\n\t</Key>\n</KeyFile>\n"
    );
    std::fs::write(path, xml).map_err(|e| SpError::Io(format!("write keyfile: {e}")))
}

fn build_key(factors: &CredentialFactors) -> Result<DatabaseKey, SpError> {
    let mut key = DatabaseKey::new().with_password(factors.password.as_str());
    if let Some(kf) = &factors.keyfile_path {
        let f = std::fs::File::open(kf).map_err(|e| SpError::Io(format!("open keyfile: {e}")))?;
        key = key
            .with_keyfile(&mut std::io::BufReader::new(f))
            .map_err(|e| SpError::Io(format!("read keyfile: {e}")))?;
    }
    Ok(key)
}

// ---------- 库的创建 / 打开 / 原子保存 ----------

/// 单位换算：sp-core 用 KiB，KDBX 存字节（keepass-ng config.rs 注释）
fn kdf_config(p: &KdfParams) -> KdfConfig {
    KdfConfig::Argon2id {
        iterations: u64::from(p.iterations),
        memory: u64::from(p.memory_kib) * 1024,
        parallelism: p.parallelism,
        version: Argon2Version::Version13,
    }
}

pub fn create_db(
    path: &Path,
    factors: &CredentialFactors,
    params: &KdfParams,
) -> Result<(), SpError> {
    params.validate()?;
    let config = DatabaseConfig {
        kdf_config: kdf_config(params),
        ..DatabaseConfig::default()
    };
    let mut db = Database::new(config);
    db.meta.database_name = Some(sp_core::GENERATOR_NAME.to_string());
    let key = build_key(factors)?;
    let mut out =
        std::fs::File::create(path).map_err(|e| SpError::Io(format!("create db: {e}")))?;
    db.save(&mut out, key)
        .map_err(|e| SpError::Format(format!("save: {e}")))
}

pub fn open_db(path: &Path, factors: &CredentialFactors) -> Result<Database, SpError> {
    let key = build_key(factors)?;
    let mut f = std::fs::File::open(path).map_err(|e| SpError::Io(format!("open db: {e}")))?;
    Database::open(&mut f, key)
        .map_err(|_| SpError::Credentials)
        .inspect_err(|_| {
            // SP-F-107 细化：凭据错误最常见；格式/版本错误在此处难以区分时
            // 保持凭据语义并提示用户用 keepassxc-cli 交叉验证
        })
}

/// 原子保存（design_docs/02）：写临时文件 → 成功后替换原文件。
/// 备份轮换（SP-N-312）由调用方在本函数成功返回后执行。
pub fn save_atomic(db: &Database, path: &Path, factors: &CredentialFactors) -> Result<(), SpError> {
    let key = build_key(factors)?;
    let tmp = path.with_extension("kdbx.tmp");
    {
        let mut out =
            std::fs::File::create(&tmp).map_err(|e| SpError::Io(format!("tmp file: {e}")))?;
        db.save(&mut out, key)
            .map_err(|e| SpError::Format(format!("save: {e}")))?;
        out.flush()
            .map_err(|e| SpError::Io(format!("flush: {e}")))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| SpError::Io(format!("atomic rename: {e}")))
}

// ---------- 条目操作 ----------

fn root_group(db: &Database) -> std::cell::Ref<'_, Group> {
    let root = db.root.borrow();
    std::cell::Ref::map(root, |r| {
        r.downcast_ref::<Group>().expect("root is a group")
    })
}

/// 把 EntryModel 写入 keepass_ng Entry
fn apply_model(entry: &mut Entry, m: &EntryModel) {
    entry.set_title(Some(&m.title));
    entry.set_username(m.username.as_deref());
    entry.set_password(m.password.as_ref().map(|s| s.as_str()));
    entry.set_url(m.url.as_deref());
    entry.set_notes(m.notes.as_deref());
    entry.set_raw_otp_value(m.otp.as_deref());
    // 标签分类（SP-F-108）：KDBX4 原生 Tags 字符串列表，整表覆写
    *entry.get_tags_mut() = m.tags.clone();
    // 先清掉旧的未保护自定义字段（保留名除外）
    for (k, _) in entry.additional_attributes() {
        let _ = entry.set_additional_attribute(&k, None);
    }
    for f in &m.custom_fields {
        let _ = entry.set_additional_attribute(&f.name, Some(f.value.as_str()));
    }
}

fn model_from_entry(e: &Entry) -> EntryModel {
    EntryModel {
        title: e.get_title().unwrap_or_default().to_string(),
        username: e.get_username().map(str::to_string),
        password: e.get_password().map(|s| Zeroizing::new(s.to_string())),
        url: e.get_url().map(str::to_string),
        notes: e.get_notes().map(str::to_string),
        custom_fields: e
            .additional_attributes()
            .into_iter()
            .map(|(name, value)| sp_core::CustomField {
                name,
                value: Zeroizing::new(value),
                protected: false, // v0.1 限制：见模块文档
            })
            .collect(),
        otp: e.get_raw_otp_value().map(str::to_string),
        tags: e.get_tags().clone(),
    }
}

pub fn add_entry(db: &mut Database, m: &EntryModel) -> Result<(), SpError> {
    m.validate()?;
    let mut entry = Entry::default();
    apply_model(&mut entry, m);
    let index = root_group(db).entries().len();
    group_add_child(&db.root, rc_refcell_node(entry), index)
        .map_err(|e| SpError::Format(format!("add entry: {e}")))
}

/// 按标题查找（v0.1 单组；uri 的条目段=标题）
pub fn find_entry(db: &Database, title: &str) -> Result<EntryModel, SpError> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    for n in g.entries() {
        let e = n.borrow();
        if let Some(e) = e.downcast_ref::<Entry>() {
            if e.get_title() == Some(title) {
                return Ok(model_from_entry(e));
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

pub fn list_titles(db: &Database) -> Vec<String> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    let mut v: Vec<String> = g
        .entries()
        .iter()
        .filter_map(|n| {
            let e = n.borrow();
            let e = e.downcast_ref::<Entry>()?;
            Some(e.get_title().unwrap_or_default().to_string())
        })
        .collect();
    v.sort();
    v
}

/// 浏览器扩展 get-logins 响应面（SP-F-706，design_docs/12 §3）：
/// 不含密码的条目摘要；密码/TOTP 仅经 find_entry/entry_totp_now 单条显式取回。
#[derive(Debug, Clone)]
pub struct EntrySummary {
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub has_otp: bool,
    /// 标签分类（SP-F-108，S1b-2）：摘要携带 tags 供 GUI/CLI 过滤
    pub tags: Vec<String>,
}

pub fn list_entry_summaries(db: &Database) -> Vec<EntrySummary> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    g.entries()
        .iter()
        .filter_map(|n| {
            let e = n.borrow();
            let e = e.downcast_ref::<Entry>()?;
            Some(EntrySummary {
                title: e.get_title().unwrap_or_default().to_string(),
                username: e.get_username().map(str::to_string),
                url: e.get_url().map(str::to_string),
                has_otp: e.get_raw_otp_value().is_some(),
                tags: e.get_tags().clone(),
            })
        })
        .collect()
}

/// 编辑：旧快照推入 history（SP-F-103，update_history 内建），再应用新模型
pub fn edit_entry(db: &mut Database, title: &str, m: &EntryModel) -> Result<(), SpError> {
    m.validate()?;
    let mut root = db.root.borrow_mut();
    let g = root.downcast_mut::<Group>().expect("root is a group");
    for n in g.entries() {
        let mut e = n.borrow_mut();
        if let Some(e) = e.downcast_mut::<Entry>() {
            if e.get_title() == Some(title) {
                e.update_history();
                apply_model(e, m);
                return Ok(());
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

/// 删除：从树移除并记录到 deleted_objects（KeePassXC 回收站语义的 v0.1 近似）
pub fn delete_entry(db: &mut Database, title: &str) -> Result<(), SpError> {
    let uuid = {
        let root = db.root.borrow();
        let g = root.downcast_ref::<Group>().expect("root is a group");
        let mut found = None;
        for n in g.entries() {
            let e = n.borrow();
            if let Some(e) = e.downcast_ref::<Entry>() {
                if e.get_title() == Some(title) {
                    found = Some(e.get_uuid());
                    break;
                }
            }
        }
        found.ok_or_else(|| SpError::NotFound(title.to_string()))?
    };
    let removed = keepass_ng::db::group_remove_node_by_uuid(&db.root, uuid)
        .map_err(|e| SpError::Format(format!("remove: {e}")))?;
    drop(removed);
    db.deleted_objects.insert(uuid, None);
    Ok(())
}

// ---------- 附件（SP-F-104） ----------

pub fn add_attachment(
    db: &mut Database,
    title: &str,
    name: &str,
    data: &[u8],
) -> Result<(), SpError> {
    let mut root = db.root.borrow_mut();
    let g = root.downcast_mut::<Group>().expect("root is a group");
    for n in g.entries() {
        let mut e = n.borrow_mut();
        if let Some(e) = e.downcast_mut::<Entry>() {
            if e.get_title() == Some(title) {
                e.attachments.insert(
                    name.to_string(),
                    Attachment {
                        data: Value::unprotected(data.to_vec()),
                    },
                );
                return Ok(());
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

pub fn get_attachment(db: &Database, title: &str, name: &str) -> Result<Vec<u8>, SpError> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    for n in g.entries() {
        let e = n.borrow();
        if let Some(e) = e.downcast_ref::<Entry>() {
            if e.get_title() == Some(title) {
                return e
                    .get_bytes(name)
                    .map(|b| b.to_vec())
                    .ok_or_else(|| SpError::NotFound(format!("{title}/{name}")));
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

/// 列出条目全部附件名（排序后；SP-F-104 补全面，CLI `sp attachment list` 消费）
pub fn list_attachments(db: &Database, title: &str) -> Result<Vec<String>, SpError> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    for n in g.entries() {
        let e = n.borrow();
        if let Some(e) = e.downcast_ref::<Entry>() {
            if e.get_title() == Some(title) {
                let mut names: Vec<String> = e.attachments.keys().cloned().collect();
                names.sort();
                return Ok(names);
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

// ---------- TOTP（SP-F-601：RFC 6238 经 keepass_ng totp feature 本地生成） ----------

/// 生成条目当前 TOTP 码（数字字符串）。
pub fn entry_totp_now(db: &Database, title: &str) -> Result<String, SpError> {
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    for n in g.entries() {
        let e = n.borrow();
        if let Some(e) = e.downcast_ref::<Entry>() {
            if e.get_title() == Some(title) {
                let totp = e
                    .get_otp()
                    .map_err(|err| SpError::NotFound(format!("invalid otp config: {err}")))?;
                let code = totp
                    .value_now()
                    .map_err(|err| SpError::Io(format!("system time: {err}")))?;
                return Ok(code.code);
            }
        }
    }
    Err(SpError::NotFound(title.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SP-F-104 补全面：list_attachments 排序输出 + 未命中 NotFound。
    /// （CLI 侧 1MB 双通道往返 e2e 见 sp-cli/tests/attachment_e2e.rs）
    #[test]
    fn list_attachments_sorted_and_missing_entry() {
        let dir = std::env::temp_dir().join(format!("sp-kdbx-att-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join("t.kdbx");
        let factors = CredentialFactors::new(Zeroizing::new("sp-test-only".into()), None);
        let params = KdfParams {
            memory_kib: 64 * 1024,
            iterations: 2,
            parallelism: 1,
        };
        create_db(&path, &factors, &params).expect("create");
        let mut db = open_db(&path, &factors).expect("open");
        add_entry(
            &mut db,
            &EntryModel {
                title: "e".into(),
                ..Default::default()
            },
        )
        .expect("add entry");
        add_attachment(&mut db, "e", "b.bin", b"B").expect("attach b");
        add_attachment(&mut db, "e", "a.bin", b"A").expect("attach a");
        assert_eq!(
            list_attachments(&db, "e").expect("list"),
            vec!["a.bin".to_string(), "b.bin".to_string()],
            "附件名须排序输出"
        );
        assert!(matches!(
            list_attachments(&db, "nope"),
            Err(SpError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
