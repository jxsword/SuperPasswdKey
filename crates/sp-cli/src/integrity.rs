//! 完整性基线与自动备份（SP-N-311/312，design_docs/09§5）。
//!
//! 边界（如实）：同用户态攻击者可同改 config 与库——本机制是"变更检测"
//! 而非"安全边界"；打开成功即静默更新基线（多设备同步防误报）。

use std::path::{Path, PathBuf};

use sp_core::SpError;

/// 计算文件 SHA256（hex 小写）
pub fn sha256_file(path: &Path) -> Result<String, SpError> {
    use sha2::{Digest as _, Sha256};
    let data = std::fs::read(path).map_err(|e| SpError::Io(format!("read for hash: {e}")))?;
    let mut h = Sha256::new();
    h.update(&data);
    let out = h.finalize();
    Ok(out.iter().map(|b| format!("{b:02x}")).collect())
}

/// 备份目录（库同目录下的 .sp-backup/）
fn backup_dir(db: &Path) -> PathBuf {
    let mut name = db
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "db".into());
    name.push_str(".sp-backup");
    db.with_file_name(name)
}

/// 保存成功后调用（原子替换完成后，复审修正 R1-5）：复制到备份目录，滚动保留 N 份、只读。
pub fn rotate_backup(db: &Path, keep: usize) -> Result<(), SpError> {
    let dir = backup_dir(db);
    std::fs::create_dir_all(&dir).map_err(|e| SpError::Io(format!("backup dir: {e}")))?;
    let stamp = chrono_like_stamp();
    let target = dir.join(format!(
        "{}-{}.kdbx.bak",
        db.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "db".into()),
        stamp
    ));
    std::fs::copy(db, &target).map_err(|e| SpError::Io(format!("backup copy: {e}")))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .attributes(0x1) // FILE_ATTRIBUTE_READONLY
            .open(&target);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o400));
    }
    prune_backups(&dir, keep)
}

fn prune_backups(dir: &Path, keep: usize) -> Result<(), SpError> {
    let mut baks: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| SpError::Io(format!("read backup dir: {e}")))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "bak").unwrap_or(false))
        .collect();
    baks.sort();
    while baks.len() > keep {
        let oldest = baks.remove(0);
        // 只读属性先解除
        #[cfg(windows)]
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .open(&oldest)
            .and_then(|f| {
                use std::os::windows::fs::OpenOptionsExt;
                drop(f);
                std::fs::OpenOptions::new()
                    .write(true)
                    .attributes(0x80)
                    .open(&oldest)
            });
        let _ = std::fs::remove_file(&oldest);
    }
    Ok(())
}

/// UTC 时间戳（本地无 chrono 依赖，用系统时间的简化实现）
fn chrono_like_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs:010}")
}

/// 打开失败时的外部修改检测（SP-N-311）：磁盘哈希≠基线 → 可疑提示
pub fn detect_external_change(db: &Path, baseline: &Option<String>) -> Option<String> {
    let Some(baseline) = baseline else {
        return None;
    };
    match sha256_file(db) {
        Ok(current) if current != *baseline => Some(current),
        _ => None,
    }
}
