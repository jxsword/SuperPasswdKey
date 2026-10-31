//! 条目模型（SP-F-102/103）：类型化字段 + 自定义字段 + history 语义。
//! 全字段加密由 KDBX4 格式层承担；本模型负责校验与映射输入。

use zeroize::Zeroizing;

use crate::error::SpError;

/// 自定义字段（protected=true 时以受保护值存储）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomField {
    pub name: String,
    pub value: Zeroizing<String>,
    pub protected: bool,
}

/// v0.1 条目模型（B1）：单条目快照（history 由适配层维护）
#[derive(Debug, Clone, Default)]
pub struct EntryModel {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<Zeroizing<String>>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub custom_fields: Vec<CustomField>,
    /// KeePass "otp" 属性（otpauth:// URL 格式，KeePassXC 兼容——DR-005）
    pub otp: Option<String>,
    /// 标签分类（S1b-2，SP-F-108）：KDBX4 原生 Tags 字符串列表，KeePassXC 互认；
    /// 校验见 validate（非空、trim 后不接收空白标签；存储顺序即写入顺序）
    pub tags: Vec<String>,
}

impl EntryModel {
    pub fn validate(&self) -> Result<(), SpError> {
        if self.title.trim().is_empty() {
            return Err(SpError::Parameter("entry title must not be empty".into()));
        }
        for t in &self.tags {
            if t.trim().is_empty() {
                return Err(SpError::Parameter("tag must not be empty or blank".into()));
            }
        }
        for f in &self.custom_fields {
            if f.name.trim().is_empty() {
                return Err(SpError::Parameter(
                    "custom field name must not be empty".into(),
                ));
            }
            // KDBX 标准字段名不可作为自定义字段（互操作保护）
            const RESERVED: [&str; 9] = [
                "Password",
                "BinaryData",
                "otp",
                "Title",
                "URL",
                "UserName",
                "Notes",
                "Additional",
                "BinaryDesc",
            ];
            if RESERVED.contains(&f.name.as_str()) {
                return Err(SpError::Parameter(format!(
                    "custom field name '{}' is reserved",
                    f.name
                )));
            }
        }
        if let Some(otp) = &self.otp {
            if !otp.starts_with("otpauth://") {
                return Err(SpError::Parameter(
                    "otp must be an otpauth:// URL (KeePassXC compatible, DR-005)".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_title_rejected() {
        let m = EntryModel {
            title: "  ".into(),
            ..Default::default()
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn reserved_custom_name_rejected() {
        let m = EntryModel {
            title: "t".into(),
            custom_fields: vec![CustomField {
                name: "Title".into(),
                value: Zeroizing::new("x".into()),
                protected: false,
            }],
            ..Default::default()
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn non_otpauth_otp_rejected() {
        let m = EntryModel {
            title: "t".into(),
            otp: Some("seed123".into()),
            ..Default::default()
        };
        assert!(m.validate().is_err());
    }
}
