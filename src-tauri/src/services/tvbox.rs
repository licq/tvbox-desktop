use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TvboxSiteRecord {
    pub site_key: String,
    pub site_name: String,
    pub api: Option<String>,
    pub ext: Option<String>,
    pub searchable: bool,
    pub quick_search: bool,
    pub filterable: bool,
    pub source_type: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TvboxParseRecord {
    pub name: String,
    pub url: String,
    pub source_type: Option<i64>,
    pub header_json: Option<String>,
    pub raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TvboxLiveRecord {
    pub group_name: Option<String>,
    pub name: String,
    pub url: String,
    pub source_type: Option<i64>,
    pub raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TvboxConfigRecords {
    pub sites: Vec<TvboxSiteRecord>,
    pub parses: Vec<TvboxParseRecord>,
    pub lives: Vec<TvboxLiveRecord>,
}

pub struct TvboxConfigParser;

impl TvboxConfigParser {
    pub fn parse(content: &str) -> Result<TvboxConfigRecords, String> {
        let root: Value =
            serde_json::from_str(content).map_err(|e| format!("TVBox配置解析失败: {}", e))?;
        let object = root
            .as_object()
            .ok_or_else(|| "TVBox配置必须是JSON对象".to_string())?;

        let sites = parse_section(object.get("sites"), "sites", parse_site_record)?;
        let parses = parse_section(object.get("parses"), "parses", parse_parse_record)?;
        let lives = parse_live_section(object.get("lives"))?;

        if sites.is_empty() && parses.is_empty() && lives.is_empty() {
            return Err("TVBox配置缺少有效的 sites、parses 或 lives 数据".to_string());
        }

        Ok(TvboxConfigRecords {
            sites,
            parses,
            lives,
        })
    }
}

fn json_to_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn parse_section<T, F>(
    value: Option<&Value>,
    section_name: &str,
    mut parse_record: F,
) -> Result<Vec<T>, String>
where
    F: FnMut(&Value) -> Result<T, String>,
{
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let items = value
        .as_array()
        .ok_or_else(|| format!("TVBox配置中的 {} 必须是数组", section_name))?;

    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            parse_record(item)
                .map_err(|err| format!("TVBox配置中的 {}[{}] 无效: {}", section_name, index, err))
        })
        .collect()
}

fn parse_site_record(value: &Value) -> Result<TvboxSiteRecord, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "站点配置必须是对象".to_string())?;
    let site_key = get_string(object.get("key")).ok_or_else(|| "缺少 key".to_string())?;
    let site_name = get_string(object.get("name")).ok_or_else(|| "缺少 name".to_string())?;

    Ok(TvboxSiteRecord {
        site_key,
        site_name,
        api: get_optional_string(object.get("api")),
        ext: get_optional_string(object.get("ext")),
        searchable: get_bool_with_default(object.get("searchable"), true),
        quick_search: get_bool_with_default(object.get("quickSearch"), false),
        filterable: get_bool_with_default(object.get("filterable"), false),
        source_type: get_type_string(object.get("type")).unwrap_or_default(),
        raw_json: json_to_string(value),
    })
}

fn parse_parse_record(value: &Value) -> Result<TvboxParseRecord, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "解析配置必须是对象".to_string())?;
    let name = get_string(object.get("name"))
        .or_else(|| get_string(object.get("jx")))
        .ok_or_else(|| "缺少 name 或 jx".to_string())?;
    let url = get_string(object.get("url")).ok_or_else(|| "缺少 url".to_string())?;

    Ok(TvboxParseRecord {
        name,
        url,
        source_type: get_optional_i64(object.get("type")),
        header_json: object.get("header").map(json_to_string),
        raw_json: json_to_string(value),
    })
}

fn parse_live_record(value: &Value) -> Result<TvboxLiveRecord, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "直播配置必须是对象".to_string())?;
    let name = get_string(object.get("name")).ok_or_else(|| "缺少 name".to_string())?;
    let url = get_string(object.get("url")).ok_or_else(|| "缺少 url".to_string())?;

    Ok(TvboxLiveRecord {
        group_name: None,
        name,
        url,
        source_type: get_optional_i64(object.get("type")),
        raw_json: json_to_string(value),
    })
}

fn parse_live_section(value: Option<&Value>) -> Result<Vec<TvboxLiveRecord>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let items = value
        .as_array()
        .ok_or_else(|| "TVBox配置中的 lives 必须是数组".to_string())?;

    let mut records = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mut parsed = parse_live_records(item)
            .map_err(|err| format!("TVBox配置中的 lives[{}] 无效: {}", index, err))?;
        records.append(&mut parsed);
    }
    Ok(records)
}

fn parse_live_records(value: &Value) -> Result<Vec<TvboxLiveRecord>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "直播配置必须是对象".to_string())?;

    if let Some(channels) = object.get("channels") {
        let group_name = get_optional_string(object.get("group"));
        return parse_grouped_live_channels(channels, group_name.as_deref());
    }

    Ok(vec![parse_live_record(value)?])
}

fn parse_grouped_live_channels(
    channels: &Value,
    group_name: Option<&str>,
) -> Result<Vec<TvboxLiveRecord>, String> {
    let items = channels
        .as_array()
        .ok_or_else(|| "channels 必须是数组".to_string())?;

    let mut records = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let object = item
            .as_object()
            .ok_or_else(|| format!("channels[{}] 必须是对象", index))?;
        let name = get_string(object.get("name")).ok_or_else(|| "缺少 name".to_string())?;
        let urls = extract_live_urls(object).ok_or_else(|| "缺少 url 或 urls".to_string())?;
        let source_type = get_optional_i64(object.get("type"));
        let raw_json = json_to_string(item);

        for url in urls {
            records.push(TvboxLiveRecord {
                group_name: group_name.map(str::to_string),
                name: name.clone(),
                url,
                source_type,
                raw_json: raw_json.clone(),
            });
        }
    }

    Ok(records)
}

fn extract_live_urls(object: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    if let Some(url) = get_optional_string(object.get("url")) {
        return Some(vec![url]);
    }

    let urls = object.get("urls")?.as_array()?;
    let extracted: Vec<String> = urls
        .iter()
        .filter_map(|value| match value {
            Value::String(url) if !url.is_empty() => Some(url.clone()),
            _ => None,
        })
        .collect();

    if extracted.is_empty() {
        None
    } else {
        Some(extracted)
    }
}

fn get_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        _ => None,
    }
}

fn get_optional_string(value: Option<&Value>) -> Option<String> {
    get_string(value).filter(|value| !value.is_empty())
}

fn get_optional_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn get_bool_with_default(value: Option<&Value>, default: bool) -> bool {
    match value {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(default),
        Some(Value::String(s)) => match s.as_str() {
            "1" | "true" | "TRUE" | "True" => true,
            "0" | "false" | "FALSE" | "False" => false,
            _ => default,
        },
        _ => default,
    }
}

fn get_type_string(value: Option<&Value>) -> Option<String> {
    get_string(value)
}

#[cfg(test)]
mod tests {
    use super::TvboxConfigParser;

    #[test]
    fn parses_minimal_single_warehouse_config() {
        let input = r#"{
            "sites": [
                {
                    "key": "demo",
                    "name": "演示站点",
                    "type": 1,
                    "api": "https://example.com/api.php/provide/vod/"
                }
            ],
            "parses": [
                {
                    "name": "默认解析",
                    "url": "https://parse.example.com/?url="
                }
            ],
            "lives": [
                {
                    "name": "直播源",
                    "url": "https://live.example.com/list.txt"
                }
            ]
        }"#;

        let parsed = TvboxConfigParser::parse(input).expect("config should parse");

        assert_eq!(parsed.sites.len(), 1);
        assert_eq!(parsed.sites[0].site_key, "demo");
        assert_eq!(parsed.sites[0].site_name, "演示站点");
        assert_eq!(
            parsed.sites[0].api.as_deref(),
            Some("https://example.com/api.php/provide/vod/")
        );
        assert_eq!(parsed.sites[0].source_type, "1");

        assert_eq!(parsed.parses.len(), 1);
        assert_eq!(parsed.parses[0].name, "默认解析");
        assert_eq!(parsed.parses[0].url, "https://parse.example.com/?url=");

        assert_eq!(parsed.lives.len(), 1);
        assert_eq!(parsed.lives[0].group_name, None);
        assert_eq!(parsed.lives[0].name, "直播源");
        assert_eq!(parsed.lives[0].url, "https://live.example.com/list.txt");
    }

    #[test]
    fn parses_grouped_live_channels() {
        let input = r#"{
            "lives": [
                {
                    "group": "redirect",
                    "channels": [
                        {
                            "name": "live",
                            "urls": [
                                "proxy://do=live&type=txt&ext=a",
                                "proxy://do=live&type=txt&ext=b"
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let parsed = TvboxConfigParser::parse(input).expect("grouped lives should parse");

        assert_eq!(parsed.lives.len(), 2);
        assert_eq!(parsed.lives[0].group_name.as_deref(), Some("redirect"));
        assert_eq!(parsed.lives[0].name, "live");
        assert_eq!(parsed.lives[0].url, "proxy://do=live&type=txt&ext=a");
        assert_eq!(parsed.lives[1].url, "proxy://do=live&type=txt&ext=b");
    }

    #[test]
    fn rejects_malformed_site_records() {
        let input = r#"{
            "sites": [
                {
                    "name": "缺少key",
                    "api": "https://example.com/api.php/provide/vod/"
                }
            ]
        }"#;

        assert!(TvboxConfigParser::parse(input).is_err());
    }

    #[test]
    fn rejects_empty_tvbox_payload() {
        let input = r#"{}"#;

        assert!(TvboxConfigParser::parse(input).is_err());
    }
}
