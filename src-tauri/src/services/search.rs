use crate::models::DoubanHotItem;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

pub struct SearchService {
    client: Client,
}

impl SearchService {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap();
        Self { client }
    }

    // ===== zxzj search =====
    pub async fn search_zxzj(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        let url = format!(
            "https://www.zxzjhd.com/vodsearch/-------------.html?wd={}&submit=",
            urlencoding::encode(title)
        );

        let html = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_zxzj_listing(&document)
    }

    fn parse_zxzj_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("li.col-md-6").map_err(|e| e.to_string())?;
        let thumb_selector = Selector::parse("a.stui-vodlist__thumb").map_err(|e| e.to_string())?;
        let title_selector = Selector::parse("h4.title a").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(thumb) = item.select(&thumb_selector).next() {
                let href = thumb.attr("href").unwrap_or("");
                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.zxzjhd.com{}", href)
                };

                let title = item.select(&title_selector)
                    .next()
                    .and_then(|a| a.attr("title").map(String::from));

                let poster = thumb.attr("data-original").map(String::from);

                // 从 URL 推断类型
                let item_type = if detail_url.contains("/movie/") {
                    "movie".to_string()
                } else if detail_url.contains("/dianshiju/") {
                    "series".to_string()
                } else if detail_url.contains("/zongyi/") {
                    "variety".to_string()
                } else if detail_url.contains("/dongman/") {
                    "anime".to_string()
                } else {
                    "movie".to_string()
                };

                results.push(DoubanHotItem {
                    source: "zxzj".to_string(),
                    source_name: "在线之家".to_string(),
                    detail_url,
                    item_type,
                    title,
                    poster,
                });
            }
        }
        Ok(results)
    }

    // ===== jpvod search =====
    pub async fn search_jpvod(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        let url = format!(
            "https://jpvod.com/search/-------------.html?wd={}&submit=",
            urlencoding::encode(title)
        );

        let html = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_jpvod_listing(&document)
    }

    fn parse_jpvod_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("a.d-block.card").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            let href = item.attr("href").unwrap_or("");
            let detail_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://jpvod.com{}", href)
            };

            let title = item.attr("title").map(String::from);

            // jpvod 详情页 URL 是 /vod/{id}.html，无法推断类型
            results.push(DoubanHotItem {
                source: "jpvod".to_string(),
                source_name: "贱贱".to_string(),
                detail_url,
                item_type: "generic".to_string(),
                title,
                poster: None,
            });
        }
        Ok(results)
    }

    // ===== xb6v search (POST + redirect) =====
    pub async fn search_xb6v(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        let search_url = "https://www.xb6v.com/e/search/1index.php";
        let body = format!(
            "show=title&tempid=1&tbname=article&mid=1&dopost=search&submit=&keyboard={}",
            urlencoding::encode(title)
        );

        let resp = self.client.post(search_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Referer", "https://www.xb6v.com/")
            .body(body)
            .send().await
            .map_err(|e| e.to_string())?;

        // 从 Location header 获取 searchid
        let location = resp.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let searchid = location.split("searchid=")
            .nth(1)
            .unwrap_or("");

        if searchid.is_empty() {
            return Ok(Vec::new());
        }

        let result_url = format!("https://www.xb6v.com/result/?searchid={}", searchid);
        let html = self.client.get(&result_url)
            .header("Referer", "https://www.xb6v.com/")
            .send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_xb6v_listing(&document)
    }

    fn parse_xb6v_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("li").map_err(|e| e.to_string())?;
        let thumb_selector = Selector::parse("a.stui-vodlist__thumb").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(thumb) = item.select(&thumb_selector).next() {
                let href = thumb.attr("href").unwrap_or("");
                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.xb6v.com{}", href)
                };

                let title = thumb.attr("title").map(String::from);
                let poster = thumb.attr("data-original").map(String::from);

                // 从 URL 推断类型
                let item_type = if href.contains("dianshiju") {
                    "series".to_string()
                } else if href.contains("ZongYi") {
                    "variety".to_string()
                } else if href.contains("donghuapian") {
                    "anime".to_string()
                } else {
                    "movie".to_string()
                };

                results.push(DoubanHotItem {
                    source: "xb6v".to_string(),
                    source_name: "小白影视".to_string(),
                    detail_url,
                    item_type,
                    title,
                    poster,
                });
            }
        }
        Ok(results)
    }

    // ===== parallel search entry =====
    pub async fn search_all(&self, title: &str) -> Vec<DoubanHotItem> {
        let (zxzj_result, jpvod_result, xb6v_result) = tokio::join!(
            self.search_zxzj(title),
            self.search_jpvod(title),
            self.search_xb6v(title)
        );

        let mut all = Vec::new();
        if let Ok(items) = zxzj_result { all.extend(items); }
        if let Ok(items) = jpvod_result { all.extend(items); }
        if let Ok(items) = xb6v_result { all.extend(items); }

        // zxzj 结果优先（可推断类型），generic 排在最后
        all.sort_by(|a, b| {
            if a.item_type == "generic" && b.item_type != "generic" {
                std::cmp::Ordering::Greater
            } else if a.item_type != "generic" && b.item_type == "generic" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });

        all
    }
}

impl Default for SearchService {
    fn default() -> Self { Self::new() }
}
