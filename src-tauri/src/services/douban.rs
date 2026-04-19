use crate::models::DoubanHot;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

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

        // Select all movie list items - Douban uses 'tr' elements in a table structure
        // Each movie is in a 'tr' with class 'item'
        let item_selector = Selector::parse("tr.item").map_err(|e| format!("Invalid selector: {}", e))?;

        // Title and link selector
        let title_selector = Selector::parse("td>a.nbg").map_err(|e| format!("Invalid selector: {}", e))?;

        // Poster image selector
        let poster_selector = Selector::parse("td>a.nbg>img").map_err(|e| format!("Invalid selector: {}", e))?;

        // Rating selector - looking for the rating in a span
        let rating_selector = Selector::parse("td.rating>span.rating_nums").map_err(|e| format!("Invalid selector: {}", e))?;

        for item in document.select(&item_selector) {
            // Extract title link
            let title_elem = item.select(&title_selector).next();
            let poster_elem = item.select(&poster_selector).next();
            let rating_elem = item.select(&rating_selector).next();

            if let Some(title_link) = title_elem {
                let name = title_link.attr("title").unwrap_or("").to_string();
                let href = title_link.attr("href").unwrap_or("").to_string();

                // Extract year from the name - format is "Title (Year)" or just "Title"
                let (clean_name, year) = Self::extract_year_from_name(&name);

                // Extract poster URL
                let poster = poster_elem.and_then(|img| {
                    img.attr("src").or(img.attr("srcset")).map(|s| s.to_string())
                });

                // Extract rating
                let rating = rating_elem.and_then(|span| {
                    span.text().collect::<String>().parse::<f64>().ok()
                });

                // Extract Douban ID from URL (e.g., /subject/36995126/)
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
                };

                items.push(hot_item);
                rank += 1;
            }
        }

        // If the table approach didn't work, try alternative selectors
        // Douban also uses divs with class 'pl2' and 'nbg' for movie links
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
                    };

                    items.push(hot_item);
                    rank += 1;
                }
            }
        }

        Ok(items)
    }

    fn extract_year_from_name(name: &str) -> (String, Option<i32>) {
        // Name format: "Movie Title (Year)" e.g., "肖申克的救赎 (1994)"
        // We need to extract the year and return the clean name
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
