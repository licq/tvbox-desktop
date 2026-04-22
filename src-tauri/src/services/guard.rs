use crate::services::tvbox::TvboxSiteRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPlayTarget {
    pub guard_key: String,
    pub site_key: String,
    pub item_id: String,
    pub source_id: String,
    pub episode_id: String,
}

pub fn guard_adapter_key(site: &TvboxSiteRecord) -> Option<String> {
    match site.api.as_deref().unwrap_or_default() {
        "csp_JpysGuard" | "csp_JPJGuard" => site.api.clone(),
        _ => None,
    }
}

pub fn is_guard_site_supported(site: &TvboxSiteRecord) -> bool {
    guard_adapter_key(site).is_some()
}

pub fn encode_guard_play_target(
    guard_key: &str,
    site_key: &str,
    item_id: &str,
    source_id: &str,
    episode_id: &str,
) -> String {
    format!(
        "guard://{}/{}/{}/{}/{}",
        percent_encode(guard_key),
        percent_encode(site_key),
        percent_encode(item_id),
        percent_encode(source_id),
        percent_encode(episode_id)
    )
}

pub fn decode_guard_play_target(value: &str) -> Option<GuardPlayTarget> {
    let trimmed = value.strip_prefix("guard://")?;
    let parts: Vec<_> = trimmed.split('/').collect();
    if parts.len() != 5 {
        return None;
    }

    Some(GuardPlayTarget {
        guard_key: percent_decode(parts[0])?,
        site_key: percent_decode(parts[1])?,
        item_id: percent_decode(parts[2])?,
        source_id: percent_decode(parts[3])?,
        episode_id: percent_decode(parts[4])?,
    })
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(byte as char),
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }

    encoded
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hi = *bytes.get(index + 1)?;
            let lo = *bytes.get(index + 2)?;
            decoded.push((from_hex(hi)? << 4) | from_hex(lo)?);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).ok()
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[cfg(test)]
mod tests {
    use super::{
        decode_guard_play_target, encode_guard_play_target, guard_adapter_key, percent_decode,
        percent_encode, is_guard_site_supported,
    };
    use crate::services::tvbox::TvboxSiteRecord;

    #[test]
    fn recognizes_supported_guard_sites() {
        let jpys = TvboxSiteRecord {
            site_key: "文采".to_string(),
            site_name: "💮文采┃秒播".to_string(),
            api: Some("csp_JpysGuard".to_string()),
            ext: None,
            searchable: true,
            quick_search: true,
            filterable: false,
            source_type: "3".to_string(),
            raw_json: r#"{"api":"csp_JpysGuard"}"#.to_string(),
        };
        let jpj = TvboxSiteRecord {
            site_key: "贱贱".to_string(),
            site_name: "🐭荐片┃P2P".to_string(),
            api: Some("csp_JPJGuard".to_string()),
            ext: None,
            searchable: true,
            quick_search: true,
            filterable: false,
            source_type: "3".to_string(),
            raw_json: r#"{"api":"csp_JPJGuard"}"#.to_string(),
        };

        assert!(is_guard_site_supported(&jpys));
        assert!(is_guard_site_supported(&jpj));
        assert_eq!(guard_adapter_key(&jpys).as_deref(), Some("csp_JpysGuard"));
        assert_eq!(guard_adapter_key(&jpj).as_deref(), Some("csp_JPJGuard"));
    }

    #[test]
    fn round_trips_guard_play_targets() {
        let encoded = encode_guard_play_target("csp_JpysGuard", "文采", "1419", "1", "1");
        let decoded = decode_guard_play_target(&encoded).expect("guard target should decode");

        assert_eq!(decoded.guard_key, "csp_JpysGuard");
        assert_eq!(decoded.site_key, "文采");
        assert_eq!(decoded.item_id, "1419");
        assert_eq!(decoded.source_id, "1");
        assert_eq!(decoded.episode_id, "1");
    }

    #[test]
    fn percent_encoding_round_trips_reserved_characters() {
        let raw = "csp/Jpys?文采=1";
        let encoded = percent_encode(raw);

        assert_eq!(percent_decode(&encoded).as_deref(), Some(raw));
    }
}
