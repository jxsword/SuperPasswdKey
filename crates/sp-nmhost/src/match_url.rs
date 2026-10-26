//! URL 匹配引擎（SP-F-706，design_docs/12 §3）：host 侧纯函数，单源可测。
//! 规则（MVP，无 PSL 依赖——多级 TLD 局限如实声明）：
//! scheme 白名单 http/https；host 归一（小写/去尾点/端口参与）；
//! 精确 host+端口 = 100 > base domain（末两段对齐，防子串误配）= 60 >
//! 条目无 scheme 的裸 host 相同 = 40；scheme 不一致扣 10；其余不匹配。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    /// 小写 host（IPv6 保留方括号）；不含端口
    pub host: String,
    pub port: Option<u16>,
}

pub const SCORE_EXACT: i32 = 100;
pub const SCORE_BASE_DOMAIN: i32 = 60;
pub const SCORE_BARE_HOST: i32 = 40;
pub const PENALTY_SCHEME_MISMATCH: i32 = 10;

/// 严格解析：`scheme://[user@]host[:port]/...`；解析失败返回 None（默认不匹配）。
pub fn parse_url(u: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = u.split_once("://")?;
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority
        .rsplit_once('@')
        .map(|(_, a)| a)
        .unwrap_or(authority);
    // IPv6：[::1]:8080
    if let Some(after) = authority.strip_prefix('[') {
        let (h, tail) = after.split_once(']')?;
        if h.is_empty() {
            return None;
        }
        let port = match tail.strip_prefix(':') {
            Some(p) => Some(p.parse().ok()?),
            None if tail.is_empty() => None,
            None => return None,
        };
        return Some(ParsedUrl {
            scheme: scheme.to_ascii_lowercase(),
            host: format!("[{h}]"),
            port,
        });
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            (h, Some(p.parse().ok()?))
        }
        _ => (authority, None),
    };
    if host.is_empty() {
        return None;
    }
    Some(ParsedUrl {
        scheme: scheme.to_ascii_lowercase(),
        host: host.trim_end_matches('.').to_ascii_lowercase(),
        port,
    })
}

fn default_port(scheme: &str) -> u16 {
    if scheme == "https" {
        443
    } else {
        80
    }
}

/// base domain = 末两段（无 PSL：co.uk 类多级 TLD 会误判，backlog）；
/// 单标签 host（如 localhost）返回 None（仅精确匹配）；IPv6 字面量整体比较。
fn base_domain(host: &str) -> Option<&str> {
    if host.starts_with('[') {
        return Some(host);
    }
    let without_tld = host.rsplit_once('.')?.0;
    Some(match without_tld.rsplit_once('.') {
        Some((_, base)) => base,
        None => without_tld,
    })
}

/// 同一 host 端点：host 相同且端口一致（双方省略视为相等；显式端口与对方
/// scheme 默认端口相等亦视为同端点）。scheme 差异不参与端点判定，只作罚分。
fn same_endpoint(entry: &ParsedUrl, page: &ParsedUrl) -> bool {
    if entry.host != page.host {
        return false;
    }
    match (entry.port, page.port) {
        (None, None) => true,
        (Some(p), None) => p == default_port(&entry.scheme) || p == default_port(&page.scheme),
        (None, Some(p)) => p == default_port(&entry.scheme) || p == default_port(&page.scheme),
        (Some(a), Some(b)) => a == b,
    }
}

/// 单条目打分：None = 不匹配
pub fn score(page: &ParsedUrl, entry_url: &str) -> Option<i32> {
    if !matches!(page.scheme.as_str(), "http" | "https") {
        return None;
    }
    let entry = match parse_url(entry_url) {
        Some(e) => {
            if !matches!(e.scheme.as_str(), "http" | "https") {
                return None;
            }
            e
        }
        // 无 scheme 的裸 host（design_docs/12 §3 规则 1）
        None => {
            let bare = entry_url
                .split(['/', '?', '#'])
                .next()?
                .trim_end_matches('.')
                .to_ascii_lowercase();
            if bare.is_empty() || bare != page.host {
                return None;
            }
            return Some(SCORE_BARE_HOST);
        }
    };
    let mut s = if same_endpoint(&entry, page) {
        SCORE_EXACT
    } else {
        let be = base_domain(&entry.host);
        let bp = base_domain(&page.host);
        match (be, bp) {
            (Some(be), Some(bp)) if be == bp && entry.port == page.port => SCORE_BASE_DOMAIN,
            _ => return None,
        }
    };
    if entry.scheme != page.scheme {
        s -= PENALTY_SCHEME_MISMATCH;
    }
    Some(s)
}

/// 对条目 URL 列表打分并按分数降序返回索引（同分保持原顺序，稳定排序）
pub fn rank(page_url: &str, entry_urls: &[Option<&str>]) -> Vec<usize> {
    let page = match parse_url(page_url) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let mut scored: Vec<(usize, i32)> = entry_urls
        .iter()
        .enumerate()
        .filter_map(|(i, u)| u.and_then(|u| score(&page, u)).map(|s| (i, s)))
        .collect();
    scored.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
    scored.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank_of(page: &str, urls: &[&str]) -> Vec<usize> {
        let v: Vec<Option<&str>> = urls.iter().map(|u| Some(*u)).collect();
        rank(page, &v)
    }

    #[test]
    fn parse_basic() {
        let u = parse_url("https://Login.Example.COM./path?q=1").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "login.example.com");
        assert_eq!(u.port, None);
    }

    #[test]
    fn parse_with_userinfo_and_port() {
        let u = parse_url("http://user:pw@host.example:8443/x").unwrap();
        assert_eq!(u.host, "host.example");
        assert_eq!(u.port, Some(8443));
    }

    #[test]
    fn parse_ipv6() {
        let u = parse_url("http://[::1]:8080/login").unwrap();
        assert_eq!(u.host, "[::1]");
        assert_eq!(u.port, Some(8080));
    }

    #[test]
    fn parse_failures() {
        for bad in ["", "example.com", "://x", "https://", "we ird://a.b"] {
            assert!(parse_url(bad).is_none(), "should fail: {bad}");
        }
    }

    #[test]
    fn exact_match_scores_highest() {
        let v = rank_of(
            "https://login.example.com/x",
            &["https://other.org/a", "https://login.example.com/b"],
        );
        assert_eq!(v, vec![1]);
    }

    #[test]
    fn base_domain_matches_subdomain_but_not_substring() {
        let s1 = score(
            &parse_url("https://login.example.com").unwrap(),
            "https://example.com/x",
        );
        assert_eq!(s1, Some(SCORE_BASE_DOMAIN));
        // 子串误配拒绝（design_docs/12 §3 反例）
        assert_eq!(
            score(
                &parse_url("https://evilexample.com").unwrap(),
                "https://example.com"
            ),
            None
        );
        assert_eq!(
            score(
                &parse_url("https://example.com").unwrap(),
                "https://evilexample.com"
            ),
            None
        );
    }

    #[test]
    fn port_mismatch_blocks_exact() {
        assert_eq!(
            score(
                &parse_url("https://example.com:8443").unwrap(),
                "https://example.com:9443"
            ),
            None
        );
        // 省略端口 = scheme 默认端口
        assert_eq!(
            score(
                &parse_url("https://example.com").unwrap(),
                "https://example.com:443"
            ),
            Some(SCORE_EXACT)
        );
    }

    #[test]
    fn scheme_mismatch_penalized() {
        assert_eq!(
            score(
                &parse_url("https://example.com").unwrap(),
                "http://example.com"
            ),
            Some(SCORE_EXACT - PENALTY_SCHEME_MISMATCH)
        );
    }

    #[test]
    fn non_http_page_never_matches() {
        assert!(rank("ftp://example.com", &[Some("https://example.com")]).is_empty());
        assert!(rank("chrome-extension://abc/x", &[Some("https://example.com")]).is_empty());
    }

    #[test]
    fn non_http_entry_never_matches() {
        assert!(rank("https://example.com", &[Some("ftp://example.com")]).is_empty());
    }

    #[test]
    fn bare_host_entry_matches_only_exact_page_host() {
        assert_eq!(
            score(&parse_url("https://example.com/x").unwrap(), "example.com"),
            Some(SCORE_BARE_HOST)
        );
        assert_eq!(
            score(
                &parse_url("https://login.example.com").unwrap(),
                "example.com"
            ),
            None
        );
    }

    #[test]
    fn rank_orders_by_score_then_stable() {
        let v = rank_of(
            "https://a.example.com/login",
            &[
                "https://other.com",       // base domain? no → 不匹配
                "http://a.example.com/x",  // exact-10
                "https://a.example.com/y", // exact
                "https://example.com/z",   // base domain
            ],
        );
        assert_eq!(v, vec![2, 1, 3]);
    }

    #[test]
    fn invalid_page_url_returns_empty() {
        assert!(rank("not a url", &[Some("https://example.com")]).is_empty());
    }
}
