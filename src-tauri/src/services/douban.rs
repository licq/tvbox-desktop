use crate::models::DoubanHot;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::time::Duration;

const DOUBAN_API_BASE: &str = "https://movie.douban.com/j/search_subjects";

#[derive(Debug, Clone)]
pub struct DoubanCategory {
    pub item_type: &'static str,
    pub type_param: &'static str,
    pub tag: &'static str,
}

pub const DOUBAN_CATEGORIES: &[DoubanCategory] = &[
    DoubanCategory { item_type: "movie",   type_param: "movie",  tag: "热门" },
    DoubanCategory { item_type: "series",  type_param: "tv",     tag: "热门" },
    DoubanCategory { item_type: "variety", type_param: "tv",    tag: "综艺" },
    DoubanCategory { item_type: "anime",   type_param: "tv",     tag: "动漫" },
];

#[derive(Debug, Deserialize)]
struct DoubanJsonItem {
    id: String,
    title: String,
    cover: String,
    rate: Option<f64>,
    episodes_info: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DoubanJsonResponse {
    subjects: Vec<DoubanJsonItem>,
}

pub struct DoubanCrawler {
    client: Client,
}

impl DoubanCrawler {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    pub async fn fetch_category(&self, category: &DoubanCategory) -> Result<Vec<DoubanHot>, String> {
        let url = format!(
            "{}?type={}&tag={}&page_limit=30&page_start=0",
            DOUBAN_API_BASE, category.type_param, category.tag
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: DoubanJsonResponse = resp.json().await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut items = Vec::new();
        for (rank, item) in json.subjects.iter().enumerate() {
            let id = item.id.parse::<i64>().unwrap_or(0);
            items.push(DoubanHot {
                id,
                name: item.title.clone(),
                year: None,
                poster: Some(item.cover.clone()),
                rating: item.rate,
                rank: (rank + 1) as i32,
                updated_at: chrono_now(),
                item_type: category.item_type.to_string(),
            });
        }
        Ok(items)
    }

    pub async fn fetch_all(&self) -> Result<Vec<DoubanHot>, String> {
        use tokio::time::sleep;
        let mut all_items = Vec::new();

        for category in DOUBAN_CATEGORIES {
            match self.fetch_category(category).await {
                Ok(items) => all_items.extend(items),
                Err(e) => log::warn!("Failed to fetch {}: {}", category.item_type, e),
            }
            // 豆瓣 API 频率限制：每次请求间隔 500ms
            sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(all_items)
    }

    pub async fn fetch_hot_list(&self) -> Result<Vec<DoubanHot>, String> {
        let url = "https://movie.douban.com/chart";
        let resp = self.client.get(url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        let html = resp.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let document = Html::parse_document(&html);
        self.parse_hot_list(&document)
    }

    fn parse_hot_list(&self, document: &Html) -> Result<Vec<DoubanHot>, String> {
        let mut items = Vec::new();
        let mut rank: i32 = 1;

        let item_selector = Selector::parse("tr.item").map_err(|e| format!("Invalid selector: {}", e))?;
        let title_selector = Selector::parse("td>a.nbg").map_err(|e| format!("Invalid selector: {}", e))?;
        let poster_selector = Selector::parse("td>a.nbg>img").map_err(|e| format!("Invalid selector: {}", e))?;
        let rating_selector = Selector::parse("td.rating>span.rating_nums").map_err(|e| format!("Invalid selector: {}", e))?;

        for item in document.select(&item_selector) {
            let title_elem = item.select(&title_selector).next();
            let poster_elem = item.select(&poster_selector).next();
            let rating_elem = item.select(&rating_selector).next();

            if let Some(title_link) = title_elem {
                let name = title_link.attr("title").unwrap_or("").to_string();
                let href = title_link.attr("href").unwrap_or("").to_string();

                let (clean_name, year) = Self::extract_year_from_name(&name);

                let poster = poster_elem.and_then(|img| {
                    img.attr("src").or(img.attr("srcset")).map(|s| s.to_string())
                });

                let rating = rating_elem.and_then(|span| {
                    span.text().collect::<String>().parse::<f64>().ok()
                });

                let douban_id = href.split("/subject/")
                    .nth(1)
                    .and_then(|s| s.trim_end_matches('/').parse::<i64>().ok())
                    .unwrap_or(0);

                let hot_item = DoubanHot {
                    id: douban_id,
                    name: clean_name,
                    year,
                    poster,
                    rating,
                    rank,
                    updated_at: chrono_now(),
                    item_type: "movie".to_string(),
                };

                items.push(hot_item);
                rank += 1;
            }
        }

        if items.is_empty() {
            let alt_item_selector = Selector::parse("div.pl2").map_err(|e| format!("Invalid selector: {}", e))?;
            let alt_title_selector = Selector::parse("a.nbg").map_err(|e| format!("Invalid selector: {}", e))?;
            let alt_img_selector = Selector::parse("a.nbg>img").map_err(|e| format!("Invalid selector: {}", e))?;
            let alt_rating_selector = Selector::parse("span.rating_nums").map_err(|e| format!("Invalid selector: {}", e))?;

            for item in document.select(&alt_item_selector) {
                let title_link = item.select(&alt_title_selector).next();
                let img_elem = item.select(&alt_img_selector).next();
                let rating_elem = item.select(&alt_rating_selector).next();

                if let Some(link) = title_link {
                    let name = link.attr("title").unwrap_or("").to_string();
                    let href = link.attr("href").unwrap_or("").to_string();

                    let (clean_name, year) = Self::extract_year_from_name(&name);

                    let poster = img_elem.and_then(|img| {
                        img.attr("src").or(img.attr("srcset")).map(|s| s.to_string())
                    });

                    let rating = rating_elem.and_then(|span| {
                        span.text().collect::<String>().parse::<f64>().ok()
                    });

                    let douban_id = href.split("/subject/")
                        .nth(1)
                        .and_then(|s| s.trim_end_matches('/').parse::<i64>().ok())
                        .unwrap_or(0);

                    let hot_item = DoubanHot {
                        id: douban_id,
                        name: clean_name,
                        year,
                        poster,
                        rating,
                        rank,
                        updated_at: chrono_now(),
                        item_type: "movie".to_string(),
                    };

                    items.push(hot_item);
                    rank += 1;
                }
            }
        }

        Ok(items)
    }

    fn extract_year_from_name(name: &str) -> (String, Option<i32>) {
        if let Some(start) = name.rfind('(') {
            if let Some(end) = name.rfind(')') {
                let year_str = &name[start+1..end];
                if year_str.len() == 4 && year_str.chars().all(|c| c.is_ascii_digit()) {
                    let year: i32 = year_str.parse().ok().unwrap_or(0);
                    if year > 1900 && year < 2100 {
                        let clean_name = name[..start].trim().to_string();
                        return (clean_name, Some(year));
                    }
                }
            }
        }
        (name.to_string(), None)
    }
}

// Helper function to get current timestamp
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs().to_string()
}

impl Default for DoubanCrawler {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DoubanSubjectScraper - WebView-based Douban subject metadata scraper
// =============================================================================
//
// 使用 Tauri WebView（真实浏览器引擎）来加载豆瓣页面，
// 以通过豆瓣的反爬虫 JavaScript 挑战（SHA-512 验证）。
// 然后通过 on_navigation 回调机制将提取的数据传回 Rust。
// =============================================================================

use crate::models::DoubanSubjectMeta;

pub struct DoubanSubjectScraper;

impl DoubanSubjectScraper {
    pub async fn scrape(app: &tauri::AppHandle, douban_id: i64) -> Result<DoubanSubjectMeta, String> {
        let url_str = format!("https://movie.douban.com/subject/{}/", douban_id);
        log::info!("[DoubanSubjectScraper] Starting scrape for douban_id={}", douban_id);

        let url: tauri::Url = url_str.parse()
            .map_err(|e| format!("Invalid URL: {}", e))?;

        // Shared state for communication between on_navigation callback and async fn
        let result = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let result_clone = result.clone();

        // Create hidden WebView window with on_navigation callback
        let webview = tauri::webview::WebviewWindowBuilder::new(
            app,
            format!("douban-scraper-{}", douban_id),
            tauri::WebviewUrl::External(url),
        )
        .title("豆瓣")
        .inner_size(1.0, 1.0) // Tiny window, off-screen
        .on_navigation(move |nav_url| {
            let nav_str = nav_url.as_str();
            // Intercept our custom communication URL
            if let Some(data_encoded) = nav_str.strip_prefix("http://scraper.internal/result?data=") {
                if let Ok(decoded) = urlencoding::decode(data_encoded) {
                    *result_clone.lock().unwrap() = Some(decoded.into_owned());
                }
                return false; // Prevent actual navigation
            }
            true
        })
        .build()
        .map_err(|e| format!("Failed to create WebView: {}", e))?;

        // Wait for page to load and anti-scraping challenge to complete
        // (SHA-512 computation + redirect to real page can take 10-15 seconds)
        log::info!("[DoubanSubjectScraper] Waiting for page load + anti-scraping challenge...");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        // Inject JS to extract metadata from DOM and communicate via navigation.
        // Uses retry loop: checks for real content repeatedly until found or timeout.
        let js = format!(r#"
            (function() {{
                var maxRetries = 25;
                var retryDelay = 1500;
                var retryCount = 0;

                var getText = function(sel) {{
                    var el = document.querySelector(sel);
                    return el ? el.textContent.trim() : '';
                }};
                var getAttr = function(sel, attr) {{
                    var el = document.querySelector(sel);
                    return el ? el.getAttribute(attr) : '';
                }};

                var sendData = function() {{
                    var title = getText('h1 span[property="v:itemreviewed"]') || getText('h1');
                    var isFinal = retryCount >= maxRetries;

                    if (!isFinal && (!title || title.indexOf('载入') >= 0 || title.indexOf('验证') >= 0)) {{
                        retryCount++;
                        setTimeout(sendData, retryDelay);
                        return;
                    }}

                    var rating = getText('.rating_num');
                    var poster = getAttr('#mainpic img', 'src');
                    var summaryText = getText('[property="v:summary"]') || getText('#link-report');
                    var infoText = getText('#info');
                    var ratingCountText = getText('.rating_sum');

                    var data = JSON.stringify({{
                        douban_id: {},
                        title: title,
                        rating: rating,
                        poster: poster,
                        summary: summaryText,
                        infoText: infoText,
                        ratingCount: ratingCountText
                    }});

                    window.location = 'http://scraper.internal/result?data=' + encodeURIComponent(data);
                }};

                setTimeout(sendData, 1000);
            }})();
        "#, douban_id);

        log::info!("[DoubanSubjectScraper] Executing JS extraction...");
        if let Err(e) = webview.eval(&js) {
            log::warn!("[DoubanSubjectScraper] eval failed: {:?}", e);
        }

        // Wait for result with timeout (enough for initial wait + all retries)
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(50);
        let data_str = loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Some(d) = result.lock().unwrap().take() {
                break d;
            }
            if tokio::time::Instant::now() > deadline {
                webview.close().ok();
                return Err("Timeout waiting for scrape result".to_string());
            }
        };

        // Clean up the WebView window
        webview.close().ok();
        log::info!("[DoubanSubjectScraper] Data received, WebView closed");

        // Parse the JSON response from the extracted data
        let v: serde_json::Value = serde_json::from_str(&data_str)
            .map_err(|e| format!("Failed to parse scraped JSON: {}", e))?;

        let title = v["title"].as_str().unwrap_or("").to_string();
        let info_text = v["infoText"].as_str().unwrap_or("");
        let summary_raw = v["summary"].as_str().unwrap_or("");

        // --- Parse fields ---
        let rating = v["rating"].as_str()
            .and_then(|s| if s.is_empty() { None } else { s.parse::<f64>().ok() });

        let rating_count = v["ratingCount"].as_str()
            .and_then(|s| {
                if s.is_empty() { return None; }
                let cleaned: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                cleaned.parse::<i64>().ok()
            });

        let poster = v["poster"].as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let summary = if summary_raw.is_empty() { None } else { Some(summary_raw.to_string()) };

        let director = extract_info_list(info_text, "导演");
        let writer = extract_info_list(info_text, "编剧");
        let actors = extract_info_list(info_text, "主演");
        let genre = extract_info_list(info_text, "类型");
        let country = extract_info_list(info_text, "制片国家/地区");
        let language = extract_info_list(info_text, "语言");
        let release_date = extract_info_list(info_text, "上映日期");
        let runtime_raw = extract_info_text(info_text, "片长")
            .map(|s| s.to_string());

        log::info!("[DoubanSubjectScraper] Success for douban_id={}: title='{}', rating={:?}, poster={:?}", douban_id, title, rating, poster.as_deref().unwrap_or("none"));

        Ok(DoubanSubjectMeta {
            douban_id,
            title,
            rating,
            rating_count,
            director,
            writer,
            actors,
            genre,
            country,
            language,
            release_date,
            runtime: runtime_raw,
            summary,
            poster,
        })
    }
}

/// 从 info 文本中提取字段列表（如 导演: 名1 / 名2 → ["名1", "名2"]）
fn extract_info_list(info_text: &str, field: &str) -> Vec<String> {
    let pattern = format!("{}:\\s*([^\n]+)", field);
    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(caps) = re.captures(info_text) {
            if let Some(m) = caps.get(1) {
                return m.as_str()
                    .split('/')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    vec![]
}

/// 从 info 文本中提取单字段值（如 片长: 123分钟 → "123分钟"）
fn extract_info_text<'a>(info_text: &'a str, field: &str) -> Option<&'a str> {
    let pattern = format!("{}:\\s*([^\n]+)", regex::escape(field));
    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(caps) = re.captures(info_text) {
            return caps.get(1).map(|m| m.as_str().trim());
        }
    }
    None
}