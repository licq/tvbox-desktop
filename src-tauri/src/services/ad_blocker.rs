/// HLS ad segment filtering.
///
/// Parses HLS playlists (m3u8) and removes segments whose URLs
/// match known ad CDN domains or URL patterns.
pub struct HlsAdBlocker;

/// Known ad CDN domain fragments and URL path patterns.
///
/// These patterns are matched against the full URL (lowercased) to identify
/// ad segments in HLS playlists. Organized by category for maintainability.
const AD_PATTERNS: &[&str] = &[
    // === Global video ad networks ===
    "doubleclick",
    "googlesyndication",
    "googletagservices",
    "googletagmanager",
    "googleadservices",
    "google-analytics",
    "googlevideo.com/ad",       // YouTube ad segments
    "pubads.g.doubleclick",
    "serverside.doubleclick",
    "adservice.google",
    "adsrvr.org",
    "adnxs.com",
    "rubiconproject",
    "openx.net",
    "criteo.com",
    "spotxchange",
    "adsafeprotected",
    "casalemedia",
    "adcolony",
    "vungle.com",

    // === Chinese ad networks (广告联盟) ===
    "allyes.com",               // 好耶广告联盟
    "adsunion.com",             // 广告联盟
    "adsense.",                 // Google Adsense (also used by Chinese sites)
    "gdt.qq.com",               // 腾讯广点通

    // === Chinese streaming platform ad domains ===
    "atm.youku.com",            // 优酷 PCDN/广告
    "cupid.iqiyi.com",          // 爱奇艺广告
    "afp.qiyi.com",             // 爱奇艺广告
    "msg.71.am",                // 爱奇艺消息推送
    "stat.youku.com",           // 优酷统计/广告
    "ad.api.3g.youku.com",      // 优酷移动广告
    "ad.mobile.youku.com",      // 优酷移动广告
    "ad.youku.com",             // 优酷广告
    "ad.v.qq.com",              // 腾讯视频广告
    "cm.l.qq.com",              // 腾讯广告
    "livep.l.qq.com",           // 腾讯直播广告
    "x.da.hunantv.com",         // 芒果TV广告
    "y.da.hunantv.com",         // 芒果TV广告
    "video.da.mgtv.com",        // 芒果TV广告
    "a.cctv.com",               // CCTV广告
    "ad.cctv.com",              // CCTV广告
    "dcads.sina.com.cn",        // 新浪广告
    "pro.letv.com",             // 乐视广告
    "adextensioncontrol.tudou.com", // 土豆广告
    "adcontrol.tudou.com",      // 土豆广告
    "analytics.ku6.com",        // 酷6广告
    "acs.56.com",               // 56我乐广告
    "adguanggao.eee114.com",    // 广告
    "cctv.adsunion.com",        // CCTV广告联盟
    "pole.6rooms.com",          // 6间房广告
    "advstat.xunlei.com",       // 迅雷看看广告
    "biz5.sandai.net",          // 迅雷广告

    // === Ad protocol / URL path patterns ===
    "/gampad/",                 // Google Ad Manager HLS ad insertion
    "/vast?",                   // VAST ad protocol
    "/vmap?",                   // VMAP ad protocol
    "/adserve/",                // Ad serving path
    "/adserver/",               // Ad server path
    ".ads.",                    // Generic ad subdomain (e.g. x.ads.cdn.com)
    "//ads.",                   // Host prefix (e.g. https://ads.example.com)

    // === URL path patterns ===
    "/ad/",                     // Common ad path
    "ad-",                      // Ad prefix in filenames (ad-1001.ts)
    "adservice",                // Ad service
    "-ads-",                    // Ads in path
    "adtrack",                  // Ad tracking
    "/gg/",                     // 广告 (Chinese "guanggao" abbreviation)
    "/tc/",                     // 推广 (Chinese "tuiguang" abbreviation)
];

impl HlsAdBlocker {
    /// Check if a URL belongs to a known ad CDN or matches an ad pattern.
    fn is_ad_url(url: &str) -> bool {
        let url_lower = url.to_lowercase();
        AD_PATTERNS
            .iter()
            .any(|&pattern| url_lower.contains(pattern))
    }

    /// Parse duration from an #EXTINF line.
    ///
    /// e.g. "#EXTINF:10.0," -> Some(10.0)
    /// e.g. "#EXTINF:10.0,This is a title" -> Some(10.0)
    fn parse_extinf_duration(line: &str) -> Option<f64> {
        let trimmed = line.trim();
        if !trimmed.starts_with("#EXTINF:") {
            return None;
        }
        // Find colon dynamically to handle any UTF-8 content
        let colon_pos = trimmed.find(':').unwrap();
        let after_colon = &trimmed[colon_pos + 1..];
        // Duration is everything before the first comma
        let comma_pos = after_colon.find(',').unwrap_or(after_colon.len());
        let duration_str = &after_colon[..comma_pos].trim();
        duration_str.parse::<f64>().ok()
    }

    /// Check if a segment's duration is anomalous compared to its neighbors.
    ///
    /// Returns true if the current duration differs from BOTH neighbors by more
    /// than 50%. A single anomalous segment amid normal ones is likely an ad.
    ///
    /// Returns false if there are fewer than 2 neighbors (not enough data).
    fn is_duration_anomalous(
        current: f64,
        prev: Option<f64>,
        next: Option<f64>,
    ) -> bool {
        let prev_diff = prev.map_or(false, |p| {
            let ratio = p / current;
            ratio < 0.5 || ratio > 2.0
        });
        let next_diff = next.map_or(false, |n| {
            let ratio = n / current;
            ratio < 0.5 || ratio > 2.0
        });
        prev_diff && next_diff
    }

    /// Remove ad segments from an HLS playlist.
    ///
    /// Scans the playlist for `#EXTINF:` + URL pairs. When a segment URL matches
    /// the ad blacklist AND its duration is anomalous compared to neighbors,
    /// both the `#EXTINF:` line and the URL line are removed.
    ///
    /// The duration anomaly check serves as a secondary validation: ads often have
    /// random/unusual durations while normal content segments have consistent durations.
    /// This reduces false positives from URL patterns that match legitimate content.
    ///
    /// Any `#EXT-X-DISCONTINUITY` line immediately preceding a removed ad segment
    /// is also removed.
    ///
    /// Master playlists (containing `#EXT-X-STREAM-INF`) are passed through
    /// unchanged here; embedded variants are cleaned before normalization in
    /// the resolver.
    pub fn filter_playlist(playlist: &str) -> String {
        if playlist.is_empty() {
            return String::new();
        }

        // Master playlists don't have EXTINF/segment pairs -- pass through
        if playlist.contains("#EXT-X-STREAM-INF") {
            return playlist.to_string();
        }

        let lines: Vec<&str> = playlist.lines().collect();
        let mut result: Vec<&str> = Vec::with_capacity(lines.len());
        let mut i = 0;

        // Collect segment info for duration analysis: (extinf_line, url_line, duration)
        let segments: Vec<(usize, usize, f64)> = {
            let mut segs = Vec::new();
            let mut si = 0;
            while si < lines.len() {
                let line = lines[si];
                if line.starts_with("#EXTINF:") {
                    let dur = Self::parse_extinf_duration(line).unwrap_or(0.0);
                    let mut url_idx = si + 1;
                    while url_idx < lines.len() {
                        let nxt = lines[url_idx].trim();
                        if nxt.is_empty() || nxt.starts_with('#') {
                            url_idx += 1;
                        } else {
                            break;
                        }
                    }
                    if url_idx < lines.len() {
                        segs.push((si, url_idx, dur));
                    }
                }
                si += 1;
            }
            segs
        };

        while i < lines.len() {
            let line = lines[i];

            if line.starts_with("#EXTINF:") {
                // Find this segment's index in our collected list
                let seg_idx = segments.iter().position(|(extinf_i, _, _)| *extinf_i == i);

                // Look ahead for the segment URL (next non-comment, non-empty line)
                let mut url_line_index = i + 1;
                while url_line_index < lines.len() {
                    let next = lines[url_line_index].trim();
                    if next.is_empty() || next.starts_with('#') {
                        url_line_index += 1;
                    } else {
                        break;
                    }
                }

                if url_line_index < lines.len() {
                    let url = lines[url_line_index].trim();
                    let is_ad = Self::is_ad_url(url);

                    // Duration anomaly check: get prev and next segment durations
                    let (prev_dur, next_dur) = if let Some(idx) = seg_idx {
                        let prev = idx.checked_sub(1).and_then(|pi| segments.get(pi)).map(|(_, _, d)| *d);
                        let next = segments.get(idx + 1).map(|(_, _, d)| *d);
                        (prev, next)
                    } else {
                        (None, None)
                    };

                    let current_dur = seg_idx.and_then(|idx| segments.get(idx)).map(|(_, _, d)| *d).unwrap_or(0.0);
                    let duration_anomaly = Self::is_duration_anomalous(current_dur, prev_dur, next_dur);

                    // Remove if URL matches ad pattern AND duration is anomalous
                    if is_ad && duration_anomaly {
                        // Remove this ad segment. Also remove any DISCONTINUITY
                        // line that was right before the EXTINF.
                        if !result.is_empty()
                            && result
                                .last()
                                .map_or(false, |l| l.contains("#EXT-X-DISCONTINUITY"))
                        {
                            result.pop();
                        }
                        // Skip EXTINF line, URL line, and any DISCONTINUITY after the segment
                        i = url_line_index + 1;
                        // Also skip DISCONTINUITY that might follow the ad segment
                        while i < lines.len() && lines[i].contains("#EXT-X-DISCONTINUITY") {
                            i += 1;
                        }
                        continue;
                    }
                }
            }

            result.push(line);
            i += 1;
        }

        let mut output = result.join("\n");

        // Preserve trailing newline if the original input had one.
        // str::lines() strips trailing newlines, so we need to restore
        // them to keep the playlist structurally identical.
        if playlist.ends_with('\n') {
            output.push('\n');
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_ad_segments_by_domain_and_duration() {
        // Both URL matches ad pattern AND duration differs from neighbors by >50%
        // ad segment has 1.0s while content segments have 10.0s
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:1.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
    }

    #[test]
    fn ad_url_without_duration_anomaly_not_filtered() {
        // URL matches ad pattern but duration is similar to neighbors (not anomalous)
        // This prevents false positives on URLs that happen to contain "ad" but are legitimate
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/ad-1001.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // ad-1001.ts matches "/ad/" in URL but duration is consistent with neighbors
        // so it should NOT be filtered (prevents false positives)
        assert!(result.contains("ad-1001.ts"));
        assert!(result.contains("seg1.ts"));
        assert!(result.contains("seg2.ts"));
    }

    #[test]
    fn duration_anomaly_only_not_filtered() {
        // Duration is anomalous but URL does not match ad patterns
        // Without URL match, should NOT be filtered (avoid false positives)
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:1.0,\nhttps://cdn.example.com/seg2.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg3.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // seg2.ts has anomalous duration but URL is clean - should NOT be filtered
        assert!(result.contains("cdn.example.com/seg2.ts"));
    }

    #[test]
    fn passes_through_clean_playlist_unchanged() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn removes_discontinuity_before_ad_segment() {
        // ad segment has anomalous duration (1.0 vs 10.0 neighbors)
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:1.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
    }

    #[test]
    fn handles_master_playlist_with_variant_urls() {
        let playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nhttps://cdn.example.com/variant.m3u8\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn handles_empty_playlist() {
        let result = HlsAdBlocker::filter_playlist("");
        assert_eq!(result, "");
    }

    #[test]
    fn handles_playlist_with_only_ads() {
        // All segments have same duration (10.0), so no duration anomaly
        // ad-cdn.example.com contains "/ad/" but duration matches neighbors
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // With no duration anomaly (all same), nothing filtered
        assert!(result.contains("#EXTM3U"));
        assert!(result.contains("#EXTINF"));
    }

    #[test]
    fn filters_ad_segments_by_url_with_duration_anomaly() {
        // URL matches ad pattern and duration differs from neighbors
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:2.0,\nhttps://cdn.example.com/ad-1001.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // ad-1001.ts matches "/ad/" AND duration 2.0 differs from 10.0 neighbors
        assert!(!result.contains("ad-1001.ts"));
        assert!(result.contains("seg1.ts"));
        assert!(result.contains("seg2.ts"));
    }

    #[test]
    fn parse_extinf_duration_tests() {
        assert_eq!(HlsAdBlocker::parse_extinf_duration("#EXTINF:10.0,"), Some(10.0));
        assert_eq!(HlsAdBlocker::parse_extinf_duration("#EXTINF:2.5,Segment Title"), Some(2.5));
        assert_eq!(HlsAdBlocker::parse_extinf_duration("#EXT-X-DISCONTINUITY"), None);
        assert_eq!(HlsAdBlocker::parse_extinf_duration("not an extinf line"), None);
    }

    #[test]
    fn is_duration_anomaly_tests() {
        // Normal case: all same duration
        assert!(!HlsAdBlocker::is_duration_anomalous(10.0, Some(10.0), Some(10.0)));
        // Anomaly: 1.0 vs neighbors 10.0 (ratio = 0.1 < 0.5)
        assert!(HlsAdBlocker::is_duration_anomalous(1.0, Some(10.0), Some(10.0)));
        // Anomaly: 25.0 vs neighbors 10.0 (ratio = 2.5 > 2.0)
        assert!(HlsAdBlocker::is_duration_anomalous(25.0, Some(10.0), Some(10.0)));
        // One neighbor matches but other doesn't - both must differ
        assert!(!HlsAdBlocker::is_duration_anomalous(1.0, Some(10.0), Some(1.0)));
        // Not enough data
        assert!(!HlsAdBlocker::is_duration_anomalous(1.0, None, Some(10.0)));
    }
}
