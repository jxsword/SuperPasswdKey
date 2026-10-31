//! 导出核心（SP-F-501~503；S1c 自 sp-cli 下沉：GUI 导出经 kffi 会话凭据复用
//! 同一实现，CLI 保留交互壳）。
//!
//! 默认 = 加密归档（zip + age 口令加密）；自校验往返确认归档可用同口令解密
//! （官方 age 工具亦可独立解密）。明文 JSON 收集同样在此（CLI --stdout 流式）。

use std::io::Write as _;
use std::path::Path;

use age::secrecy::SecretString;
use serde::Serialize;

use sp_core::SpError;

#[derive(Serialize)]
pub struct ExportEntry {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<(String, Vec<u8>)>>,
    pub history_count: usize,
}

/// 导出摘要（GUI 展示/CLI 打印共用；机密值不进此结构）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportSummary {
    pub entries: usize,
    pub attachments: usize,
    pub history_total: usize,
    pub warnings: Vec<String>,
}

/// 收集全量（条目 + 附件 + history 计数）
pub fn collect(
    db: &crate::engine::db::Database,
) -> Result<(Vec<ExportEntry>, Vec<String>), SpError> {
    use crate::engine::db::{Entry, Group, Node};
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

fn summarize(entries: &[ExportEntry], warnings: Vec<String>) -> ExportSummary {
    ExportSummary {
        entries: entries.len(),
        attachments: entries
            .iter()
            .map(|e| e.attachments.as_ref().map(|a| a.len()).unwrap_or(0))
            .sum(),
        history_total: entries.iter().map(|e| e.history_count).sum(),
        warnings,
    }
}

/// 导出摘要文案（CLI 打印/GUI 帮助共用；不含机密值）
pub fn summary_text(s: &ExportSummary) -> String {
    format!(
        "Export summary: {} entries (incl. custom fields, otp, attachments {}, history records {}).\n\
         NOT included: passkeys (not stored in v0.1), trash/deleted objects.\n\
         Handle plaintext exports as top-secret: delete immediately after use; never send via email or upload.",
        s.entries, s.attachments, s.history_total
    )
}

/// 加密归档导出（zip + age 口令加密 + 自校验往返）。passphrase 由调用方
/// 提供（CLI 交互两遍确认；GUI 对话框两遍输入），不落任何持久层。
pub fn export_encrypted_archive(
    db: &crate::engine::db::Database,
    out: &Path,
    passphrase: &str,
) -> Result<ExportSummary, SpError> {
    let (entries, warnings) = collect(db)?;
    let summary = summarize(&entries, warnings);

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

    let recipient = age::scrypt::Recipient::new(SecretString::from(passphrase.to_string()));
    let encrypted =
        age::encrypt(&recipient, &archive).map_err(|e| SpError::Io(format!("age encrypt: {e}")))?;
    // 自校验：确认归档可用同口令解密（F10 验收：官方 age 工具亦可独立解密）
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_string()));
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
    Ok(summary)
}

/// 明文导出 JSON 收集（SP-F-502；CLI --stdout 流式消费，零临时文件）
pub fn collect_plaintext_json(db: &crate::engine::db::Database) -> Result<Vec<u8>, SpError> {
    let (entries, _) = collect(db)?;
    serde_json::to_vec_pretty(&entries).map_err(|e| SpError::Io(format!("serialize: {e}")))
}
