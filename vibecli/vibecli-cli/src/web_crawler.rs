// Web Crawler — crawl websites, documentation sites, and sitemaps for RAG ingestion.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Crawl configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub max_pages: usize,
    pub max_depth: usize,
    pub delay_ms: u64,              // delay between requests (rate limiting)
    pub timeout_secs: u64,
    pub user_agent: String,
    pub respect_robots_txt: bool,
    pub follow_external: bool,       // follow links to other domains
    pub include_patterns: Vec<String>, // URL patterns to include (regex)
    pub exclude_patterns: Vec<String>, // URL patterns to exclude
    pub allowed_content_types: Vec<String>,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 100,
            max_depth: 5,
            delay_ms: 500,
            timeout_secs: 30,
            user_agent: "VibeCody-Crawler/1.0".to_string(),
            respect_robots_txt: true,
            follow_external: false,
            include_patterns: vec![],
            exclude_patterns: vec![
                r"\.(png|jpg|jpeg|gif|svg|ico|css|js|woff|woff2|ttf|eot|mp4|mp3|pdf|zip|tar|gz)$".to_string(),
            ],
            allowed_content_types: vec![
                "text/html".to_string(),
                "text/plain".to_string(),
                "application/json".to_string(),
                "text/markdown".to_string(),
                "application/xml".to_string(),
            ],
        }
    }
}

/// A crawled page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPage {
    pub url: String,
    pub title: Option<String>,
    pub content: String,       // extracted plain text
    pub raw_html: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub depth: usize,
    pub links: Vec<String>,    // outgoing links found
    pub crawled_at: String,
    pub word_count: usize,
    pub headers: HashMap<String, String>,
}

/// Crawl job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Crawl job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub seed_url: String,
    pub status: CrawlStatus,
    pub pages: Vec<CrawledPage>,
    pub pages_crawled: usize,
    pub pages_skipped: usize,
    pub errors: Vec<CrawlError>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub config: CrawlConfig,
}

/// Crawl error record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlError {
    pub url: String,
    pub error: String,
    pub status_code: Option<u16>,
}

/// Robots.txt parser (simple)
#[derive(Debug, Clone)]
pub struct RobotsTxt {
    pub disallowed: Vec<String>,
    pub allowed: Vec<String>,
    pub crawl_delay: Option<u64>,
    pub sitemaps: Vec<String>,
}

impl RobotsTxt {
    /// Parse robots.txt content
    pub fn parse(content: &str) -> Self {
        let mut disallowed = Vec::new();
        let mut allowed = Vec::new();
        let mut crawl_delay = None;
        let mut sitemaps = Vec::new();
        let mut in_our_section = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let lower = line.to_lowercase();
            if lower.starts_with("user-agent:") {
                let agent = line[11..].trim().to_lowercase();
                in_our_section = agent == "*" || agent.contains("vibecody");
            } else if lower.starts_with("sitemap:") {
                sitemaps.push(line[8..].trim().to_string());
            } else if in_our_section {
                if lower.starts_with("disallow:") {
                    let path = line[9..].trim();
                    if !path.is_empty() {
                        disallowed.push(path.to_string());
                    }
                } else if lower.starts_with("allow:") {
                    let path = line[6..].trim();
                    if !path.is_empty() {
                        allowed.push(path.to_string());
                    }
                } else if lower.starts_with("crawl-delay:") {
                    crawl_delay = line[12..].trim().parse::<u64>().ok();
                }
            }
        }

        Self { disallowed, allowed, crawl_delay, sitemaps }
    }

    /// Check if a URL path is allowed
    pub fn is_allowed(&self, path: &str) -> bool {
        // Check allowed rules first (more specific wins)
        for allow in &self.allowed {
            if path.starts_with(allow) {
                return true;
            }
        }
        for disallow in &self.disallowed {
            if path.starts_with(disallow) {
                return false;
            }
        }
        true
    }
}

/// Sitemap parser
pub struct SitemapParser;

impl SitemapParser {
    /// Extract URLs from a sitemap XML
    pub fn parse(xml: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let mut in_loc = false;

        for line in xml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("<loc>") && trimmed.ends_with("</loc>") {
                let url = &trimmed[5..trimmed.len()-6];
                urls.push(url.to_string());
            } else if trimmed.starts_with("<loc>") {
                in_loc = true;
            } else if trimmed.ends_with("</loc>") && in_loc {
                in_loc = false;
            } else if trimmed.contains("<loc>") && trimmed.contains("</loc>") {
                if let Some(start) = trimmed.find("<loc>") {
                    if let Some(end) = trimmed.find("</loc>") {
                        urls.push(trimmed[start+5..end].to_string());
                    }
                }
            }
        }

        urls
    }

    /// Parse a sitemap index (contains links to other sitemaps)
    pub fn parse_index(xml: &str) -> Vec<String> {
        let mut sitemap_urls = Vec::new();
        for line in xml.lines() {
            let trimmed = line.trim();
            if trimmed.contains("<loc>") && trimmed.contains("</loc>") {
                if let Some(start) = trimmed.find("<loc>") {
                    if let Some(end) = trimmed.find("</loc>") {
                        let url = &trimmed[start+5..end];
                        if url.contains("sitemap") || url.ends_with(".xml") {
                            sitemap_urls.push(url.to_string());
                        }
                    }
                }
            }
        }
        sitemap_urls
    }
}

/// Extract links from HTML content
pub fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let mut links = Vec::new();
    let base = base_url.trim_end_matches('/');

    // Extract domain from base URL for relative URL resolution
    let domain = {
        let without_proto = base.split("://").nth(1).unwrap_or(base);
        let host = without_proto.split('/').next().unwrap_or(without_proto);
        let proto = base.split("://").next().unwrap_or("https");
        format!("{}://{}", proto, host)
    };

    // Find href attributes
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(idx) = lower[pos..].find("href=") {
        let start = pos + idx + 5;
        let quote = if html.as_bytes().get(start) == Some(&b'"') { '"' }
                   else if html.as_bytes().get(start) == Some(&b'\'') { '\'' }
                   else { pos = start + 1; continue; };

        let url_start = start + 1;
        if let Some(end) = html[url_start..].find(quote) {
            let href = &html[url_start..url_start + end];
            let resolved = resolve_url(href, base, &domain);
            if let Some(url) = resolved {
                if url.starts_with("http") {
                    links.push(url);
                }
            }
            pos = url_start + end;
        } else {
            pos = start + 1;
        }
    }

    links
}

/// Resolve a potentially relative URL
fn resolve_url(href: &str, base: &str, domain: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") || href.starts_with("mailto:") || href.starts_with("data:") {
        return None;
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }

    if href.starts_with("//") {
        return Some(format!("https:{}", href));
    }

    if href.starts_with('/') {
        return Some(format!("{}{}", domain, href));
    }

    // Relative URL
    let base_path = base.rfind('/').map(|i| &base[..i]).unwrap_or(base);
    Some(format!("{}/{}", base_path, href))
}

/// Normalize a URL for deduplication
pub fn normalize_url(url: &str) -> String {
    let mut normalized = url.to_string();
    // Remove fragment
    if let Some(hash) = normalized.find('#') {
        normalized.truncate(hash);
    }
    // Remove trailing slash
    if normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }
    // Remove common tracking params
    if let Some(query) = normalized.find('?') {
        let params = &normalized[query+1..];
        let filtered: Vec<&str> = params.split('&')
            .filter(|p| {
                let key = p.split('=').next().unwrap_or("");
                !["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term", "ref", "fbclid", "gclid"].contains(&key)
            })
            .collect();
        if filtered.is_empty() {
            normalized.truncate(query);
        } else {
            normalized = format!("{}?{}", &normalized[..query], filtered.join("&"));
        }
    }
    normalized
}

/// Save crawl results to disk
pub fn save_crawl_results(result: &CrawlResult, output_dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let filename = format!("crawl-{}.json",
        result.seed_url.replace("://", "-").replace('/', "_").replace('.', "_"));
    let path = output_dir.join(filename);
    let data = serde_json::to_string_pretty(result)?;
    std::fs::write(&path, data)?;
    Ok(path)
}

fn now_ts() -> String {
    format!("{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_config_default() {
        let config = CrawlConfig::default();
        assert_eq!(config.max_pages, 100);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.delay_ms, 500);
        assert!(config.respect_robots_txt);
        assert!(!config.follow_external);
    }

    #[test]
    fn test_robots_txt_parse() {
        let robots = "User-agent: *\nDisallow: /admin/\nDisallow: /private/\nAllow: /admin/public/\nCrawl-delay: 2\nSitemap: https://example.com/sitemap.xml";
        let parsed = RobotsTxt::parse(robots);
        assert_eq!(parsed.disallowed.len(), 2);
        assert_eq!(parsed.allowed.len(), 1);
        assert_eq!(parsed.crawl_delay, Some(2));
        assert_eq!(parsed.sitemaps.len(), 1);
    }

    #[test]
    fn test_robots_txt_is_allowed() {
        let robots = RobotsTxt::parse("User-agent: *\nDisallow: /admin/\nAllow: /admin/public/");
        assert!(robots.is_allowed("/page"));
        assert!(!robots.is_allowed("/admin/secret"));
        assert!(robots.is_allowed("/admin/public/file"));
    }

    #[test]
    fn test_sitemap_parse() {
        let xml = "<urlset>\n  <url><loc>https://example.com/page1</loc></url>\n  <url><loc>https://example.com/page2</loc></url>\n</urlset>";
        let urls = SitemapParser::parse(xml);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com/page1");
    }

    #[test]
    fn test_extract_links() {
        let html = r#"<a href="/about">About</a> <a href="https://external.com">Ext</a> <a href="page.html">Page</a>"#;
        let links = extract_links(html, "https://example.com/docs");
        assert!(links.contains(&"https://example.com/about".to_string()));
        assert!(links.contains(&"https://external.com".to_string()));
        assert!(links.contains(&"https://example.com/page.html".to_string()));
    }

    #[test]
    fn test_extract_links_skips_fragments() {
        let html = r##"<a href="#section">Anchor</a> <a href="javascript:void(0)">JS</a> <a href="mailto:a@b.com">Mail</a>"##;
        let links = extract_links(html, "https://example.com");
        assert!(links.is_empty());
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("https://example.com/page#section"), "https://example.com/page");
        assert_eq!(normalize_url("https://example.com/page/"), "https://example.com/page");
        assert_eq!(normalize_url("https://example.com/page?utm_source=test&key=val"), "https://example.com/page?key=val");
        assert_eq!(normalize_url("https://example.com/page?utm_source=x"), "https://example.com/page");
    }

    #[test]
    fn test_resolve_url_absolute() {
        assert_eq!(resolve_url("https://other.com/page", "https://example.com", "https://example.com"), Some("https://other.com/page".to_string()));
    }

    #[test]
    fn test_resolve_url_relative() {
        assert_eq!(resolve_url("/about", "https://example.com/docs/page", "https://example.com"), Some("https://example.com/about".to_string()));
        assert_eq!(resolve_url("page2.html", "https://example.com/docs/page1", "https://example.com"), Some("https://example.com/docs/page2.html".to_string()));
    }

    #[test]
    fn test_resolve_url_protocol_relative() {
        assert_eq!(resolve_url("//cdn.example.com/file", "https://example.com", "https://example.com"), Some("https://cdn.example.com/file".to_string()));
    }

    #[test]
    fn test_crawl_status_serialization() {
        let status = CrawlStatus::Failed("timeout".to_string());
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("failed"));
        let back: CrawlStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
    }

    #[test]
    fn test_sitemap_index_parse() {
        let xml = "<sitemapindex>\n<sitemap><loc>https://example.com/sitemap-1.xml</loc></sitemap>\n<sitemap><loc>https://example.com/sitemap-2.xml</loc></sitemap>\n</sitemapindex>";
        let sitemaps = SitemapParser::parse_index(xml);
        assert_eq!(sitemaps.len(), 2);
    }
}
