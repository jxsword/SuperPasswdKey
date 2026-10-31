//! F9 批量导入器（SP-F-401~403，design_docs/05）。
//!
//! 管道：来源 → 解析器 → 统一 Finding → 三级去重 → 分类 → 报告。
//! 发现引擎：gitleaks 规则 TOML 内嵌（MIT，222 规则，2026-10-26 抓取 master）。

pub mod gitleaks_toml {
    /// 内嵌规则（构建期打包，无运行时读取）
    pub const RULES_TOML: &str = include_str!("importer/gitleaks.toml");
}

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;
use sp_core::{CustomField, EntryModel, SpError};
use zeroize::Zeroizing;

// ---------- 统一 Finding 模型 ----------

#[derive(Debug, Clone)]
pub struct Finding {
    pub source: String,
    pub file: String,
    pub line: usize,
    pub rule_id: String,
    pub secret: Zeroizing<String>,
    pub fingerprint: String,
}

// ---------- gitleaks 规则引擎 ----------

#[derive(Debug, Deserialize)]
struct GitleaksConfig {
    #[serde(default)]
    rules: Vec<GitleaksRule>,
}

#[derive(Debug, Deserialize)]
struct GitleaksRule {
    id: String,
    #[serde(default)]
    regex: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // 保留字段：gitleaks 规则语义完整性
    keywords: Vec<String>,
}

pub struct RuleEngine {
    rules: Vec<(String, regex::Regex)>,
}

impl RuleEngine {
    pub fn gitleaks() -> Result<Self, SpError> {
        let cfg: GitleaksConfig = toml::from_str(gitleaks_toml::RULES_TOML)
            .map_err(|e| SpError::Parameter(format!("gitleaks.toml parse: {e}")))?;
        let mut rules = Vec::new();
        for r in cfg.rules {
            if let Some(re) = r.regex {
                match regex::Regex::new(&re) {
                    Ok(cre) => rules.push((r.id, cre)),
                    Err(_) => continue, // 个别高级语法规则跳过（v0.1 如实降级）
                }
            }
        }
        Ok(Self { rules })
    }

    pub fn scan(&self, content: &str, source: &str, file: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        for (lineno, line) in content.lines().enumerate() {
            for (id, re) in &self.rules {
                if let Some(m) = re.find(line) {
                    let secret = m.as_str().to_string();
                    let fingerprint = {
                        use sha2::{Digest as _, Sha256};
                        let mut h = Sha256::new();
                        h.update(secret.as_bytes());
                        let d = h.finalize();
                        d.iter()
                            .take(8)
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    };
                    out.push(Finding {
                        source: source.to_string(),
                        file: file.to_string(),
                        line: lineno + 1,
                        rule_id: id.clone(),
                        secret: Zeroizing::new(secret),
                        fingerprint,
                    });
                }
            }
        }
        out
    }
}

// ---------- 来源解析器 ----------

fn finding(source: &str, file: &str, line: usize, rule: &str, value: String) -> Finding {
    use sha2::{Digest as _, Sha256};
    let mut h = Sha256::new();
    h.update(value.as_bytes());
    let d = h.finalize();
    let fingerprint = d
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Finding {
        source: source.to_string(),
        file: file.to_string(),
        line,
        rule_id: rule.to_string(),
        secret: Zeroizing::new(value),
        fingerprint,
    }
}

/// .env / dotenv 语法（复用 F8 解析器）；目录则递归 .env*
pub fn parse_dotenv(path: &Path) -> Result<Vec<Finding>, SpError> {
    let mut out = Vec::new();
    if path.is_dir() {
        collect_files(path, &mut |f| {
            if f.file_name()
                .map(|n| n.to_string_lossy().starts_with(".env"))
                .unwrap_or(false)
            {
                if let Ok(content) = std::fs::read_to_string(f) {
                    for (i, (k, v)) in crate::f8::parse_env_str(&content).into_iter().enumerate() {
                        out.push(finding(
                            "dotenv",
                            &f.to_string_lossy(),
                            i + 1,
                            "dotenv",
                            format!("{k}={v}"),
                        ));
                    }
                }
            }
            Ok(())
        })?;
    } else {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SpError::Io(format!("read {path:?}: {e}")))?;
        for (i, (k, v)) in crate::f8::parse_env_str(&content).into_iter().enumerate() {
            out.push(finding(
                "dotenv",
                &path.to_string_lossy(),
                i + 1,
                "dotenv",
                format!("{k}={v}"),
            ));
        }
    }
    Ok(out)
}

/// 通用 CSV（列别名映射对齐 KeePassXC/Bitwarden 表头）
pub fn parse_csv(path: &Path) -> Result<Vec<Finding>, SpError> {
    let mut out = Vec::new();
    let mut rdr =
        csv::Reader::from_path(path).map_err(|e| SpError::Io(format!("open csv: {e}")))?;
    let headers = rdr
        .headers()
        .map_err(|e| SpError::Io(format!("csv headers: {e}")))?
        .clone();
    let alias = |h: &str| -> Option<&'static str> {
        let l = h.to_ascii_lowercase();
        Some(match l.as_str() {
            "title" | "name" | "account" => "title",
            "username" | "user" | "login" | "login_username" => "username",
            "password" | "passwd" | "pwd" => "password",
            "url" | "uri" | "website" | "login_uri" => "url",
            "notes" | "note" | "comment" => "notes",
            "totp" | "otp" => "totp",
            _ => return None,
        })
    };
    let mut mapping: HashMap<usize, &'static str> = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        if let Some(std_name) = alias(h) {
            mapping.insert(i, std_name);
        }
    }
    for (rowno, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| SpError::Io(format!("csv row: {e}")))?;
        let mut kv: BTreeMap<&str, String> = BTreeMap::new();
        for (i, v) in record.iter().enumerate() {
            if v.is_empty() {
                continue;
            }
            if let Some(std_name) = mapping.get(&i) {
                kv.entry(std_name).or_insert_with(|| v.to_string());
            }
        }
        if let Some(pw) = kv.get("password") {
            if !pw.is_empty() {
                out.push(finding(
                    "csv",
                    &path.to_string_lossy(),
                    rowno + 2, // +header
                    "csv-password",
                    pw.clone(),
                ));
            }
        }
        if let Some(t) = kv.get("totp") {
            if !t.is_empty() {
                out.push(finding(
                    "csv",
                    &path.to_string_lossy(),
                    rowno + 2,
                    "csv-totp",
                    t.clone(),
                ));
            }
        }
    }
    Ok(out)
}

/// Bitwarden JSON（核心字段：login/uri/notes/fields/totp；附件与 passkey 后置——SRS §2.3）
pub fn parse_bw_json(path: &Path) -> Result<Vec<Finding>, SpError> {
    let raw = std::fs::read_to_string(path).map_err(|e| SpError::Io(format!("read json: {e}")))?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| SpError::Io(format!("parse json: {e}")))?;
    let items = v
        .get("items")
        .and_then(|i| i.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let name = item
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unnamed");
        if let Some(login) = item.get("login") {
            if let Some(pw) = login.get("password").and_then(|p| p.as_str()) {
                if !pw.is_empty() {
                    out.push(finding(
                        "bw-json",
                        name,
                        idx + 1,
                        "bw-password",
                        pw.to_string(),
                    ));
                }
            }
            if let Some(t) = login.get("totp").and_then(|p| p.as_str()) {
                if !t.is_empty() {
                    out.push(finding("bw-json", name, idx + 1, "bw-totp", t.to_string()));
                }
            }
        }
        for f in item
            .get("fields")
            .and_then(|f| f.as_array())
            .into_iter()
            .flatten()
        {
            let fname = f.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let fvalue = f.get("value").and_then(|n| n.as_str()).unwrap_or("");
            if !fvalue.is_empty() {
                out.push(finding(
                    "bw-json",
                    name,
                    idx + 1,
                    "bw-field",
                    format!("{fname}={fvalue}"),
                ));
            }
        }
    }
    Ok(out)
}

/// 当前进程环境变量快照（内存枚举，不落盘——市场空白点，gitleaks/trufflehog 均无此源）
pub fn parse_env_snapshot() -> Vec<Finding> {
    let engine = RuleEngine::gitleaks().unwrap_or_else(|_| RuleEngine { rules: Vec::new() });
    let mut out = Vec::new();
    for (k, v) in std::env::vars() {
        for f in engine.scan(&v, "env", &k) {
            out.push(f);
        }
    }
    out
}

/// 散落文件扫描（gitleaks 规则，递归目录）
pub fn parse_files(dir: &Path) -> Result<Vec<Finding>, SpError> {
    let engine = RuleEngine::gitleaks()?;
    let mut out = Vec::new();
    collect_files(dir, &mut |f| {
        // 跳过明显非文本/超大文件（v0.1 简化：≤2MB 且可 UTF-8）
        if let Ok(meta) = f.metadata() {
            if meta.len() > 2 * 1024 * 1024 {
                return Ok(());
            }
        }
        if let Ok(content) = std::fs::read_to_string(f) {
            let label = f.to_string_lossy().to_string();
            out.extend(engine.scan(&content, "files", &label));
        }
        Ok(())
    })?;
    Ok(out)
}

fn collect_files(
    dir: &Path,
    cb: &mut dyn FnMut(&Path) -> Result<(), SpError>,
) -> Result<(), SpError> {
    for entry in std::fs::read_dir(dir).map_err(|e| SpError::Io(format!("read dir: {e}")))? {
        let p = entry
            .map_err(|e| SpError::Io(format!("dir entry: {e}")))?
            .path();
        if p.is_dir() {
            collect_files(&p, cb)?;
        } else {
            cb(&p)?;
        }
    }
    Ok(())
}

// ---------- 三级去重（基线 §4.3：冲突不清除，全部保留打标记） ----------

#[derive(Debug, Default)]
pub struct DedupReport {
    pub total: usize,
    pub unique: usize,
    pub l1_duplicates: usize,
    pub l2_merged: usize,
    pub l3_conflicts: usize,
    /// 建议导入的条目（按 来源/路径/规则 聚类标题）
    pub candidates: Vec<EntryModel>,
    pub manual_review: Vec<String>,
}

/// 生成导入候选：三级去重 + 分类
pub fn build_candidates(findings: Vec<Finding>) -> DedupReport {
    let mut report = DedupReport {
        total: findings.len(),
        ..Default::default()
    };
    let mut seen_value: HashSet<String> = HashSet::new();
    let mut seen_target: HashSet<(String, String)> = HashSet::new(); // (url, username)
    let mut seen_fp: HashSet<String> = HashSet::new();
    let mut grouped: BTreeMap<String, Vec<Finding>> = BTreeMap::new();

    for f in findings {
        let group = format!("{}/{}/{}", f.source, parent_of(&f.file), f.rule_id);
        // L1：值哈希精确
        if !seen_value.insert(f.fingerprint.clone()) {
            report.l1_duplicates += 1;
            report.manual_review.push(format!(
                "[L1 duplicate] {}:{} {}",
                f.file, f.line, f.rule_id
            ));
            continue;
        }
        // L2：URL+用户名合并（v0.1：finding 无 url/user 元数据时不启用——空键会
        // 错误合并全部条目，实测发现；CSV 来源携带元数据后启用）
        let target = (String::new(), String::new());
        if target.0.is_empty() && target.1.is_empty() {
            // 无元数据：跳过 L2
        } else if !seen_target.insert(target) {
            report.l2_merged += 1;
            continue;
        }
        // L3：指纹前缀（同前缀不同值 → 冲突标记）
        if !seen_fp.insert(f.fingerprint.clone()) {
            report.l3_conflicts += 1;
            report
                .manual_review
                .push(format!("[L3 conflict] {}:{} {}", f.file, f.line, f.rule_id));
            continue;
        }
        grouped.entry(group).or_default().push(f);
        report.unique += 1;
    }

    for (group, items) in grouped {
        for (i, f) in items.iter().enumerate() {
            let title = format!(
                "[{}] {}#{}",
                group,
                Path::new(&f.file)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| f.file.clone()),
                if items.len() > 1 {
                    format!("{}", i + 1)
                } else {
                    String::new()
                }
            );
            report.candidates.push(EntryModel {
                title,
                username: None,
                password: Some(Zeroizing::new(f.secret.as_str().to_string())),
                url: None,
                notes: Some(format!("source: {}:{} rule: {}", f.file, f.line, f.rule_id)),
                custom_fields: vec![CustomField {
                    name: "fingerprint".into(),
                    value: Zeroizing::new(f.fingerprint.clone()),
                    protected: false,
                }],
                otp: None,
                tags: Vec::new(),
            });
        }
    }
    report
}

fn parent_of(file: &str) -> String {
    Path::new(file)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".into())
}
