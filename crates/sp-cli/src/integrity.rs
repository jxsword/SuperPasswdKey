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
    rotate_backup_with(db, keep, &chrono_like_stamp())
}

/// 可注入时间戳版本（单测确定性）；同一秒内多次保存由毫秒分量区分
/// （原实现秒级时间戳——同秒保存互相覆盖备份，2026-10-30 修复）。
pub fn rotate_backup_with(db: &Path, keep: usize, stamp: &str) -> Result<(), SpError> {
    let dir = backup_dir(db);
    std::fs::create_dir_all(&dir).map_err(|e| SpError::Io(format!("backup dir: {e}")))?;
    let target = dir.join(format!(
        "{}-{}.kdbx.bak",
        db.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "db".into()),
        stamp
    ));
    std::fs::copy(db, &target).map_err(|e| SpError::Io(format!("backup copy: {e}")))?;
    // 只读：原实现经 OpenOptions.attributes() 仅在创建时生效（copy 已建目标→
    // 静默无效，Windows 缺口 2026-10-30 修复）；set_readonly=FILE_ATTRIBUTE_READONLY。
    if let Err(e) = mark_readonly(&target) {
        eprintln!("[warn] backup readonly: {e}");
    }
    prune_backups(&dir, keep)
}

fn mark_readonly(path: &Path) -> Result<(), SpError> {
    #[cfg(windows)]
    {
        // std::os::windows::fs::PermissionsExt 仍 unstable（#152956）——
        // 基于现有 metadata 的 set_readonly（= FILE_ATTRIBUTE_READONLY）
        let mut perms = std::fs::metadata(path)
            .map_err(|e| SpError::Io(format!("backup meta: {e}")))?
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(path, perms)
            .map_err(|e| SpError::Io(format!("backup readonly: {e}")))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o400))
            .map_err(|e| SpError::Io(format!("backup readonly: {e}")))
    }
}

fn chrono_like_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:010}.{:03}", d.as_secs(), d.subsec_millis())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SP-N-312：11 次保存 → 滚动保留 10 份（最旧被清）+ 备份只读。
    /// 时间戳注入（rotate_backup_with）保证确定性——原秒级时间戳下同秒保存
    /// 会互相覆盖备份（2026-10-30 修复口径）。
    #[test]
    fn rotate_keeps_ten_oldest_pruned_and_readonly() {
        let dir = std::env::temp_dir().join(format!("sp-integrity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let db = dir.join("v.kdbx");
        for i in 0..11u32 {
            std::fs::write(&db, format!("content-{i}")).expect("write db");
            rotate_backup_with(&db, 10, &format!("{i:04}")).expect("rotate");
        }
        let bdir = backup_dir(&db);
        let count = std::fs::read_dir(&bdir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "bak").unwrap_or(false))
            .count();
        assert_eq!(count, 10, "须滚动保留 10 份");
        assert!(!bdir.join("v-0000.kdbx.bak").exists(), "最旧备份须被清除");
        let newest = bdir.join("v-0010.kdbx.bak");
        assert!(newest.exists(), "最新备份须在");
        assert_eq!(std::fs::read(&newest).expect("read"), b"content-10");
        assert!(
            std::fs::metadata(&newest)
                .expect("meta")
                .permissions()
                .readonly(),
            "备份须只读（Windows FILE_ATTRIBUTE_READONLY / unix 0400）"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SP-N-311：detect_external_change——基线相符 → None；不符/无基线 → 提示。
    #[test]
    fn detect_external_change_semantics() {
        let dir = std::env::temp_dir().join(format!("sp-integrity-detect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let db = dir.join("d.kdbx");
        std::fs::write(&db, b"v1").expect("write");
        let h1 = sha256_file(&db).expect("hash");
        assert_eq!(
            detect_external_change(&db, &Some(h1.clone())),
            None,
            "基线相符"
        );
        std::fs::write(&db, b"v2").expect("write v2");
        assert!(
            detect_external_change(&db, &Some(h1)).is_some(),
            "哈希不符须提示"
        );
        assert!(
            detect_external_change(&db, &None).is_none(),
            "无基线（首次打开）不提示——成功后回写基线才有检测能力"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
