//! F8 纯逻辑组件（SP-F-301，design_docs/04）：.env 解析、遮蔽过滤器、变量名净化。
//! 进程生成点在 `spawner.rs`（用户显式指定的子命令，argv 直 exec）。
//! .env 语法对齐 1Password op 规则（基线 §3.2#6）。
//!
//! 注：run/inject 编排依赖 spawner.rs（待用户落盘），届时移除本模块 dead_code 标注。

use std::io::Write;
use std::path::Path;

use zeroize::Zeroizing;

use sp_core::SpError;

/// 系统关键变量保护名单（SP-F-301：永不覆盖）
#[allow(dead_code)] // run/inject 接线后使用
const PROTECTED_VARS: &[&str] = &["PATH", "HOME", "USER", "TERM", "SHELL", "LC_ALL", "LANG"];

#[allow(dead_code)] // run/inject 接线后使用
pub fn is_protected(key: &str) -> bool {
    PROTECTED_VARS.contains(&key) || key.starts_with("XDG_") || key.starts_with("LC_")
}

/// POSIX 变量名净化（SP-F-301：对齐 bws --uuids-as-keynames 语义）
#[allow(dead_code)] // run/inject 接线后使用
pub fn posix_name(key: &str) -> String {
    let mut out: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

// ---------- .env 解析 ----------

/// 变量查找函数类型（文件内已解析变量 → 进程环境 回退）
pub type VarLookup = fn(&[(String, String)], &str) -> Option<String>;

#[allow(dead_code)] // run/inject 接线后使用（H4 完成）
pub fn parse_env_file(path: &Path) -> Result<Vec<(String, String)>, SpError> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| SpError::Io(format!("read env file: {e}")))?;
    Ok(parse_env_str(&raw))
}

pub fn parse_env_str(input: &str) -> Vec<(String, String)> {
    fn lookup(parsed: &[(String, String)], name: &str) -> Option<String> {
        parsed
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var(name).ok())
    }

    let mut out: Vec<(String, String)> = Vec::new();
    let mut lines = input.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(eq) = trimmed.find('=') else {
            continue;
        };
        let key = trimmed[..eq].trim().to_string();
        if key.is_empty() {
            continue;
        }
        let mut value = trimmed[eq + 1..].to_string();
        // 多行值：引号奇偶扫描整个累积值（跨行状态；修正逐行 toggle 误判）
        let opening_quote = matches!(value.trim_start().chars().next(), Some('"') | Some('\''));
        if opening_quote && quote_parity_open(&value) {
            while quote_parity_open(&value) {
                match lines.next() {
                    Some(next) => {
                        value.push('\n');
                        value.push_str(next);
                    }
                    None => break,
                }
            }
        }
        let parsed = unquote_and_expand(&value, &out, lookup);
        out.push((key, parsed));
    }
    out
}

fn quote_parity_open(s: &str) -> bool {
    let mut in_quote = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_quote => {
                let _ = chars.next();
            }
            '"' | '\'' => in_quote = !in_quote,
            _ => {}
        }
    }
    in_quote
}

fn unquote_and_expand(value: &str, parsed: &[(String, String)], lookup: VarLookup) -> String {
    let v = value.trim();
    if let Some(rest) = v.strip_prefix('\'') {
        let inner = rest.strip_suffix('\'').unwrap_or(rest);
        return inner.to_string(); // 单引号：字面量
    }
    let s = if let Some(rest) = v.strip_prefix('"') {
        rest.strip_suffix('"').unwrap_or(rest)
    } else {
        v
    };
    // $VAR / ${VAR} 展开 + \$ 转义
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('$') => out.push('$'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            '$' => {
                let mut name = String::new();
                let mut brace = false;
                if chars.peek() == Some(&'{') {
                    brace = true;
                    chars.next();
                }
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' {
                        name.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if brace {
                    let _ = chars.next();
                }
                if name.is_empty() {
                    out.push('$');
                } else if let Some(val) = lookup(parsed, &name) {
                    out.push_str(&val);
                }
            }
            other => out.push(other),
        }
    }
    out
}

// ---------- 遮蔽过滤器（SP-N-306：100% 替换，跨 chunk 边界） ----------

#[allow(dead_code)] // run/inject 接线后使用
pub struct Masker {
    secrets: Vec<Vec<u8>>,
    tail: Vec<u8>,
    /// 回看缓冲 = 最长秘密 - 1（跨边界匹配保证，design_docs/04 §2）
    holdback: usize,
}

impl Masker {
    #[allow(dead_code)]
    pub fn new(secrets: Vec<Zeroizing<String>>) -> Self {
        let raw: Vec<Vec<u8>> = secrets.iter().map(|s| s.as_bytes().to_vec()).collect();
        let max_len = raw.iter().map(|s| s.len()).max().unwrap_or(0);
        Self {
            secrets: raw,
            tail: Vec::new(),
            holdback: max_len.saturating_sub(1),
        }
    }

    #[allow(dead_code)]
    fn find_first(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        for secret in &self.secrets {
            if secret.is_empty() {
                continue;
            }
            if let Some(pos) = find_sub(&self.tail, secret) {
                if best.map(|(p, _)| pos < p).unwrap_or(true) {
                    best = Some((pos, secret.len()));
                }
            }
        }
        best
    }

    #[allow(dead_code)]
    pub fn filter_chunk(&mut self, chunk: &[u8], out: &mut impl Write) -> std::io::Result<()> {
        self.tail.extend_from_slice(chunk);
        while let Some((pos, len)) = self.find_first() {
            out.write_all(&self.tail[..pos])?;
            out.write_all(b"<redacted>")?;
            self.tail.drain(..pos + len);
        }
        // 只保留回看缓冲，其余立即写出
        if self.tail.len() > self.holdback {
            let emit_to = self.tail.len() - self.holdback;
            out.write_all(&self.tail[..emit_to])?;
            self.tail.drain(..emit_to);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn finish(&mut self, out: &mut impl Write) -> std::io::Result<()> {
        while let Some((pos, len)) = self.find_first() {
            out.write_all(&self.tail[..pos])?;
            out.write_all(b"<redacted>")?;
            self.tail.drain(..pos + len);
        }
        out.write_all(&self.tail)?;
        self.tail.clear();
        Ok(())
    }
}

#[allow(dead_code)]
fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufWriter;

    fn drain(mut m: Masker, chunks: &[&[u8]]) -> String {
        let mut buf = Vec::new();
        {
            let mut w = BufWriter::new(&mut buf);
            for c in chunks {
                m.filter_chunk(c, &mut w).unwrap();
            }
            m.finish(&mut w).unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn env_parse_basic_and_comments() {
        let env = "A=1\n# comment\nB = two words\n\nC='literal $X'\n";
        let v = parse_env_str(env);
        assert_eq!(v[0], ("A".into(), "1".into()));
        assert_eq!(v[1], ("B".into(), "two words".into()));
        assert_eq!(v[2], ("C".into(), "literal $X".into()));
    }

    #[test]
    fn env_parse_expansion_and_escape() {
        let v = parse_env_str("A=hello\nB=$A-world\nC=${A}_x\nD=\\$A\n");
        assert_eq!(v[1], ("B".into(), "hello-world".into()));
        assert_eq!(v[2], ("C".into(), "hello_x".into()));
        assert_eq!(v[3], ("D".into(), "$A".into()));
    }

    #[test]
    fn env_parse_multiline_quoted() {
        let v = parse_env_str("A=\"line1\nline2\"\nB=after\n");
        assert_eq!(v[0], ("A".into(), "line1\nline2".into()));
        assert_eq!(v[1], ("B".into(), "after".into()));
    }

    #[test]
    fn env_parse_later_vars_shadow_earlier() {
        let v = parse_env_str("A=first\nB=$A\nA=second\nC=$A\n");
        assert_eq!(v[1], ("B".into(), "first".into()));
        assert_eq!(v[3], ("C".into(), "second".into()));
    }

    #[test]
    fn posix_name_rules() {
        assert_eq!(posix_name("Valid_Name_1"), "Valid_Name_1");
        assert_eq!(posix_name("1abc"), "_1abc");
        assert_eq!(posix_name("my-key.name"), "my_key_name");
        assert_eq!(posix_name(""), "_");
        assert_eq!(posix_name("sp://db/pw"), "sp___db_pw");
    }

    #[test]
    fn masker_basic_replacement() {
        let m = Masker::new(vec![Zeroizing::new("SECRETVALUE".into())]);
        assert_eq!(
            drain(m, &[b"before SECRETVALUE after".as_slice()]),
            "before <redacted> after"
        );
    }

    /// 关键用例：秘密跨 chunk 边界（1 字节步进），必须 100% 遮蔽（SP-N-306）
    #[test]
    fn masker_cross_boundary_never_leaks() {
        let secret = "SUPERSECRETVALUE123";
        for step in 1..=7usize {
            let m = Masker::new(vec![Zeroizing::new(secret.into())]);
            let chunks: Vec<&[u8]> = secret
                .as_bytes()
                .chunks(step.max(1))
                .map(|c| c as &[u8])
                .chain(std::iter::once(b" tail".as_slice()))
                .collect();
            let out = drain(m, &chunks);
            assert!(
                !out.contains("SECRET") && out.contains("<redacted>"),
                "step={step} leaked: {out}"
            );
        }
    }

    #[test]
    fn masker_multiple_secrets_and_no_match() {
        let m = Masker::new(vec![
            Zeroizing::new("AAA".into()),
            Zeroizing::new("BBBBB".into()),
        ]);
        let out = drain(
            m,
            &[b"no secrets here, just AAAA and BBBBB and plain".as_slice()],
        );
        assert!(!out.contains("BBBBB"), "{out}");
        assert!(out.contains("<redacted>"), "{out}");
        assert!(out.contains("plain"), "{out}");
    }

    #[test]
    fn masker_empty_is_passthrough() {
        let m = Masker::new(vec![]);
        let out = drain(m, &[b"nothing to hide".as_slice()]);
        assert_eq!(out, "nothing to hide");
    }

    #[test]
    fn is_protected_list() {
        assert!(is_protected("PATH"));
        assert!(is_protected("XDG_CONFIG_HOME"));
        assert!(is_protected("LC_ALL"));
        assert!(!is_protected("DB_PASSWORD"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::io::BufWriter;

    proptest! {
        /// 任意字节级输入下 .env 解析器不崩溃（F9/H7 属性测试）
        #[test]
        fn env_parse_never_panics(input in ".*") {
            let _ = parse_env_str(&input);
        }

        /// 任意分块下遮蔽过滤器不泄漏（SP-N-306 属性化）
        #[test]
        fn masker_never_leaks_across_chunks(
            secret in "[A-Za-z0-9]{8,24}",
            filler in "[a-z ]{0,40}",
            step in 1usize..16,
        ) {
            let m = Masker::new(vec![Zeroizing::new(secret.clone())]);
            let payload = format!("{filler}{secret}{filler}{secret}{filler}");
            let mut buf = Vec::new();
            {
                let mut mm = m;
                let mut w = BufWriter::new(&mut buf);
                for c in payload.as_bytes().chunks(step) {
                    mm.filter_chunk(c, &mut w).unwrap();
                }
                mm.finish(&mut w).unwrap();
            }
            let out = String::from_utf8(buf).unwrap();
            prop_assert!(!out.contains(&secret), "leaked at step {step}: {out}");
            prop_assert_eq!(out.matches("<redacted>").count(), 2);
        }
    }
}
