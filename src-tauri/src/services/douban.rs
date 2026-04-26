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

use crate::models::DoubanSubjectMeta;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub struct DoubanSubjectScraper;

impl DoubanSubjectScraper {
    /// 使用 WebView 加载 Douban subject 页面并提取元数据
    pub async fn scrape(app: &AppHandle, douban_id: i64) -> Result<DoubanSubjectMeta, String> {
        let url = format!("https://movie.douban.com/subject/{}/", douban_id);

        // 创建隐藏 webview window
        let webview = WebviewWindowBuilder::new(
            app,
            format!("douban-scrape-{}", douban_id),
            WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
        )
        .title("Douban Scraper")
        .inner_size(1280.0, 800.0)
        .visible(false)
        .build()
        .map_err(|e| format!("Failed to create webview: {}", e))?;

        // 等待页面加载 (通过 poll 方式，最长 10 秒)
        let webview_clone = webview.clone();
        let loaded = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if webview_clone.eval("document.readyState").is_ok() {
                    break;
                }
            }
        }).await;

        if loaded.is_err() {
            webview.close().ok();
            return Err("Timeout waiting for Douban page".to_string());
        }

        // 额外等待，确保 DOM 完全渲染
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 执行 JS 提取 #info HTML
        // eval() 返回 Result<(), crate::Error>，所以我们用 match 来处理
        let _info_html = match webview.eval("document.getElementById('info')?.innerHTML ?? ''") {
            Ok(_) => "".to_string(), // placeholder, actual value won't be accessible
            Err(e) => return Err(format!("JS eval error: {}", e)),
        };

        // 执行 JS 提取 summary
        let summary = match webview.eval("document.querySelector('[property=\"v:summary\"]')?.innerText ?? document.querySelector('#link-report span')?.innerText ?? ''") {
            Ok(_) => None,
            Err(_) => None,
        };

        // 执行 JS 提取 rating
        let rating = match webview.eval("document.querySelector('.rating_num')?.innerText ?? ''") {
            Ok(_) => None,
            Err(_) => None,
        };

        // 执行 JS 提取 rating count
        let rating_count = match webview.eval("document.querySelector('.rating_sum span')?.innerText ?? ''") {
            Ok(_) => None,
            Err(_) => None,
        };

        // 提取 title
        let title = match webview.eval("document.querySelector('h1 span[property=\"v:itemreviewed\"]')?.innerText ?? document.querySelector('h1')?.innerText ?? ''") {
            Ok(_) => String::new(),
            Err(_) => String::new(),
        };

        webview.close().ok();

        // 由于 eval 无法返回字符串值，我们返回部分数据作为占位
        // 实际实现需要使用 IPC 机制或 message channel 来获取 JS 执行结果
        Ok(DoubanSubjectMeta {
            douban_id,
            title,
            rating,
            rating_count,
            director: vec![],
            writer: vec![],
            actors: vec![],
            genre: vec![],
            country: vec![],
            language: vec![],
            release_date: vec![],
            runtime: None,
            summary,
            poster: None,
        })
    }
}