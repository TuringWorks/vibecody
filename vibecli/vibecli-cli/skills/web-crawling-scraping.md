---
triggers: ["web crawling", "web scraping", "spider", "sitemap", "robots.txt", "link extraction", "content extraction"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# Web Crawling & Scraping

When building web crawlers and scrapers:

1. **robots.txt compliance** — Always fetch and respect `/robots.txt` before crawling. Parse `Disallow`, `Allow`, `Crawl-delay`, and `Sitemap` directives. Identify your crawler with a descriptive `User-Agent` string. Use libraries like `robotexclusionrulesparser` (Python) or `robots-txt` (Rust) for parsing. Violating robots.txt can lead to IP bans and legal issues.

2. **Rate limiting and polite crawling** — Enforce a minimum delay between requests to the same domain (1-5 seconds for general sites, respect `Crawl-delay` if specified). Use exponential backoff on 429 (Too Many Requests) and 503 responses. Limit concurrent connections per domain to 1-2. Crawl during off-peak hours when possible. Set reasonable timeouts (30s connect, 60s read).

3. **Sitemap parsing** — Check for sitemaps at `/sitemap.xml`, `/sitemap_index.xml`, and URLs listed in robots.txt. Parse XML sitemaps to discover all URLs with their `lastmod`, `changefreq`, and `priority` attributes. Handle sitemap indexes (sitemaps of sitemaps). Use `lastmod` dates to prioritize recently updated pages for incremental crawling.

4. **URL normalization** — Canonicalize URLs before deduplication: lowercase the scheme and host, resolve relative paths, remove default ports (80/443), sort query parameters, remove fragment identifiers, decode unreserved percent-encoded characters, and remove trailing slashes consistently. This prevents crawling the same page multiple times under different URL forms.

5. **Duplicate detection** — Maintain a visited URL set using a bloom filter for memory efficiency on large crawls. For content deduplication, compute SimHash or MinHash of extracted text to detect near-duplicate pages. Use URL normalization before checking the visited set. Store seen content hashes to skip pages that differ only in boilerplate.

6. **Content extraction** — Use readability algorithms (Mozilla Readability, trafilatura, newspaper3k) to extract main article content, stripping navigation, ads, and boilerplate. Preserve document structure (headings, lists, tables). Extract metadata (title, author, date, description) from HTML meta tags, Open Graph, and JSON-LD. Fall back to raw text extraction when readability fails.

7. **JavaScript rendering** — For SPAs and JS-rendered content, use headless browsers (Playwright, Puppeteer) or browser-as-a-service (Browserless, ScrapingBee). Wait for network idle or specific selectors before extracting content. Use Playwright's `page.content()` after rendering. Cache rendered pages to avoid re-rendering. Consider if the API behind the SPA is directly accessible as a faster alternative.

8. **Structured data extraction** — Parse JSON-LD (`<script type="application/ld+json">`), Microdata, and RDFa for structured information (products, articles, events, organizations). JSON-LD is the most common and easiest to parse. Use CSS selectors or XPath for site-specific structured extraction. Build extraction schemas that map selectors to output fields.

9. **Incremental crawling** — Store the `Last-Modified` and `ETag` headers from responses. On subsequent crawls, send `If-Modified-Since` and `If-None-Match` headers to get 304 (Not Modified) responses for unchanged pages. Track crawl timestamps per URL. Prioritize re-crawling pages with higher change frequency. Use sitemap `lastmod` dates to skip unchanged pages entirely.

10. **Crawl scheduling** — Use a priority queue (URL frontier) that balances breadth-first exploration with depth-first following of important paths. Implement domain-based politeness queues so rate limits per domain don't block other domains. Prioritize URLs by estimated importance (PageRank-like scoring, depth from seed, sitemap priority).

11. **Error handling** — Implement retry logic with exponential backoff for transient errors (5xx, timeouts, connection resets). Log and skip permanent errors (404, 410). Handle redirect chains (limit to 5-10 hops). Detect and handle soft 404s (200 status but error page content). Monitor error rates per domain and pause crawling if errors spike.

12. **Proxy rotation** — For large-scale crawling, rotate through a pool of proxy IPs to avoid rate limiting and IP bans. Use residential proxies for sites with aggressive bot detection. Implement proxy health checking and automatic rotation on ban detection. Distribute requests across proxies using round-robin or least-recently-used strategies. Track success rates per proxy.
