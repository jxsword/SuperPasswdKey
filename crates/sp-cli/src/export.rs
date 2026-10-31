//! F10 批量导出（SP-F-501~503，design_docs/06）——CLI 交互壳。
//!
//! 核心实现自 S1c 起下沉 sp-kdbx::export（GUI 经 kffi 会话凭据复用同一实现）；
//! 本模块只保留 CLI 特有部分：主密码/口令交互、摘要打印、--stdout 流式。
//!
//! 默认 = 加密归档（zip + age 口令加密）；
//! 明文 = 显式 flag + 重输主密码 + 警告，--stdout 流式（零临时文件）。

use std::io::Write as _;
use std::path::Path;

use sp_core::SpError;

/// 加密归档导出（默认）：zip + age 口令（交互两遍确认）
pub fn export_encrypted(db: &sp_kdbx::engine::db::Database, out: &Path) -> Result<(), SpError> {
    let pw1 = crate::commands::read_secret("Archive passphrase: ")?;
    let pw2 = crate::commands::read_secret("Confirm passphrase: ")?;
    if pw1.as_str() != pw2.as_str() {
        return Err(SpError::Parameter("passphrases do not match".into()));
    }
    let summary = sp_kdbx::export::export_encrypted_archive(db, out, pw1.as_str())?;
    println!("{}", sp_kdbx::export::summary_text(&summary));
    println!(
        "Archive written to {} (age-encrypted; decryptable with the official age tool).",
        out.display()
    );
    for w in &summary.warnings {
        eprintln!("warning: {w}");
    }
    Ok(())
}

/// 明文导出（JSON，SP-F-502 三重防护；--stdout 流式，零临时文件）
pub fn export_plaintext_stdout(db: &sp_kdbx::engine::db::Database) -> Result<(), SpError> {
    let json = sp_kdbx::export::collect_plaintext_json(db)?;
    // 流式：stdout 直写（Base64 附件为可读性保留 JSON 内嵌；无临时文件）
    std::io::stdout()
        .write_all(&json)
        .map_err(|e| SpError::Io(format!("stdout: {e}")))?;
    let (count, attach_count) = {
        // 摘要复用 collect（值不外泄，仅计数）
        let (entries, _) = sp_kdbx::export::collect(db)?;
        (
            entries.len(),
            entries
                .iter()
                .map(|e| e.attachments.as_ref().map(|a| a.len()).unwrap_or(0))
                .sum::<usize>(),
        )
    };
    eprintln!(
        "{}",
        sp_kdbx::export::summary_text(&sp_kdbx::export::ExportSummary {
            entries: count,
            attachments: attach_count,
            history_total: 0,
            warnings: vec![],
        })
    );
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
