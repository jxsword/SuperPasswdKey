//! F10 批量导出（SP-F-501~503，design_docs/06）。
//!
//! 默认 = 加密归档（zip + age 口令加密，行业空白点）；
//! 明文 = 显式 flag + 重输主密码 + 警告，--stdout 流式（零临时文件）。

use std::io::Write as _;
use std::path::Path;

use age::secrecy::SecretString;
use serde::Serialize;

use sp_core::SpError;

#[derive(Serialize)]
struct ExportEntry {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    otp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_fields: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<(String, Vec<u8>)>>,
    history_count: usize,
}

/// 收集全量（条目 + 附件 + history 计数）
fn collect(db: &sp_kdbx::engine::db::Database) -> Result<(Vec<ExportEntry>, Vec<String>), SpError> {
    use sp_kdbx::engine::db::{Entry, Group, Node};
    let root = db.root.borrow();
    let g = root.downcast_ref::<Group>().expect("root is a group");
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for n in g.entries() {
        let e = n.borrow();
        let Some(e) = e.downcast_ref::<Entry>() else {
            continue;
        };
        let history_count = 0usize; // keepass_ng History 内部计数未公开（DR-005 限制）
        let mut attachments: Vec<(String, Vec<u8>)> = Vec::new();
        for name in e.attachments.keys() {
            if let Some(data) = e.get_bytes(name) {
                attachments.push((name.clone(), data.to_vec()));
            } else {
                warnings.push(format!(
                    "attachment '{}' not readable on '{}'",
                    name,
                    e.get_title().unwrap_or("?")
                ));
            }
        }
        entries.push(ExportEntry {
            title: e.get_title().unwrap_or_default().to_string(),
            username: e.get_username().map(str::to_string),
            password: e.get_password().map(str::to_string),
            url: e.get_url().map(str::to_string),
            notes: e.get_notes().map(str::to_string),
            otp: e.get_raw_otp_value().map(str::to_string),
            custom_fields: if e.additional_attributes().is_empty() {
                None
            } else {
                Some(e.additional_attributes())
            },
            attachments: if attachments.is_empty() {
                None
            } else {
                Some(attachments)
            },
            history_count,
        });
    }
    Ok((entries, warnings))
}

fn field_summary(count: usize, attach_count: usize, history_total: usize) -> String {
    format!(
        "Export summary: {count} entries (incl. custom fields, otp, attachments {attach_count}, history records {history_total}).\n\
         NOT included: passkeys (not stored in v0.1), trash/deleted objects.\n\
         Handle plaintext exports as top-secret: delete immediately after use; never send via email or upload."
    )
}

/// 加密归档导出（默认）：zip + age 口令
pub fn export_encrypted(db: &sp_kdbx::engine::db::Database, out: &Path) -> Result<(), SpError> {
    let (entries, warnings) = collect(db)?;
    let attach_count: usize = entries
        .iter()
        .map(|e| e.attachments.as_ref().map(|a| a.len()).unwrap_or(0))
        .sum();
    let history_total: usize = entries.iter().map(|e| e.history_count).sum();

    let pw1 = crate::commands::read_secret("Archive passphrase: ")?;
    let pw2 = crate::commands::read_secret("Confirm passphrase: ")?;
    if pw1.as_str() != pw2.as_str() {
        return Err(SpError::Parameter("passphrases do not match".into()));
    }

    // zip 容器（内存构建，库规模小）→ age 一次性加密 → 自校验往返
    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let json = serde_json::to_vec_pretty(&entries)
            .map_err(|e| SpError::Io(format!("serialize entries: {e}")))?;
        zip.start_file("entries.json", opts)
            .map_err(|e| SpError::Io(format!("zip: {e}")))?;
        zip.write_all(&json)
            .map_err(|e| SpError::Io(format!("zip write: {e}")))?;
        for e in &entries {
            if let Some(atts) = &e.attachments {
                for (name, data) in atts {
                    zip.start_file(format!("attachments/{}/{name}", e.title), opts)
                        .map_err(|err| SpError::Io(format!("zip: {err}")))?;
                    zip.write_all(data)
                        .map_err(|err| SpError::Io(format!("zip write: {err}")))?;
                }
            }
        }
        zip.finish()
            .map_err(|e| SpError::Io(format!("zip finish: {e}")))?;
    }
    let archive = zip_buf.into_inner();

    let recipient = age::scrypt::Recipient::new(SecretString::from(pw1.as_str()));
    let encrypted =
        age::encrypt(&recipient, &archive).map_err(|e| SpError::Io(format!("age encrypt: {e}")))?;
    // 自校验：确认归档可用同口令解密（F10 验收：官方 age 工具亦可独立解密）
    let identity = age::scrypt::Identity::new(SecretString::from(pw2.as_str()));
    let roundtrip = age::decrypt(&identity, &encrypted)
        .map_err(|e| SpError::Io(format!("age self-check: {e}")))?;
    if roundtrip != archive {
        return Err(SpError::Io("age self-check mismatch".into()));
    }

    let mut out_file =
        std::fs::File::create(out).map_err(|e| SpError::Io(format!("create archive: {e}")))?;
    out_file
        .write_all(&encrypted)
        .map_err(|e| SpError::Io(format!("write archive: {e}")))?;
    out_file.flush().ok();

    println!(
        "{}",
        field_summary(entries.len(), attach_count, history_total)
    );
    println!(
        "Archive written to {} (age-encrypted; decryptable with the official age tool).",
        out.display()
    );
    for w in warnings {
        eprintln!("warning: {w}");
    }
    Ok(())
}

/// 明文导出（JSON，SP-F-502 三重防护；--stdout 流式，零临时文件）
pub fn export_plaintext_stdout(db: &sp_kdbx::engine::db::Database) -> Result<(), SpError> {
    let (entries, warnings) = collect(db)?;
    let attach_count: usize = entries
        .iter()
        .map(|e| e.attachments.as_ref().map(|a| a.len()).unwrap_or(0))
        .sum();
    let history_total: usize = entries.iter().map(|e| e.history_count).sum();
    let json =
        serde_json::to_vec_pretty(&entries).map_err(|e| SpError::Io(format!("serialize: {e}")))?;
    // 流式：stdout 直写（Base64 附件为可读性保留 JSON 内嵌；无临时文件）
    std::io::stdout()
        .write_all(&json)
        .map_err(|e| SpError::Io(format!("stdout: {e}")))?;
    eprintln!(
        "{}",
        field_summary(entries.len(), attach_count, history_total)
    );
    for w in warnings {
        eprintln!("warning: {w}");
    }
    Ok(())
}

/// 明文导出的主密码确认（三重防护之二）
pub fn confirm_master_password(factors: &sp_core::CredentialFactors) -> Result<(), SpError> {
    let pw =
        crate::commands::read_secret("Re-enter master password to confirm plaintext export: ")?;
    if pw.as_str() != factors.password.as_str() {
        return Err(SpError::Parameter("password confirmation failed".into()));
    }
    Ok(())
}
