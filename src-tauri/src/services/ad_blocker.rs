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

    /// Detect consecutive double DISCONTINUITY markers that indicate ad insertion.
    ///
    /// When ads are inserted into a stream, the playlist typically shows:
    /// ```text
    /// #EXTINF:... (content segment)
    /// #EXT-X-DISCONTINUITY
    /// #EXT-X-DISCONTINUITY   <- double = ad break starts
    /// #EXTINF:... (ad segment 1)
    /// #EXTINF:... (ad segment 2)
    /// #EXT-X-DISCONTINUITY   <- single = ad break ends, content resumes
    /// ```
    ///
    /// Returns true if `lines[i]` and `lines[i+1]` are both `#EXT-X-DISCONTINUITY`.
    fn is_double_discontinuity(lines: &[&str], i: usize) -> bool {
        i + 1 < lines.len()
            && lines[i].contains("#EXT-X-DISCONTINUITY")
            && lines[i + 1].contains("#EXT-X-DISCONTINUITY")
    }

    /// Remove ad segments from an HLS playlist.
    ///
    /// Scans the playlist for `#EXTINF:` + URL pairs. When a segment URL matches
    /// the ad blacklist, both the `#EXTINF:` line and the URL line are removed.
    /// Any `#EXT-X-DISCONTINUITY` line immediately preceding a removed ad segment
    /// is also removed.
    ///
    /// Also detects consecutive double `#EXT-X-DISCONTINUITY` markers which
    /// indicate ad insertion points. All segments between double DISCONTINUITY
    /// and the next single DISCONTINUITY are treated as ad segments.
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

        while i < lines.len() {
            let line = lines[i];

            // Check for double DISCONTINUITY marker indicating ad start
            if Self::is_double_discontinuity(&lines, i) {
                // Skip the double DISCONTINUITY lines
                i += 2;

                // Now skip all segments until we hit a single DISCONTINUITY (ad end)
                // or end of playlist
                while i < lines.len() {
                    let current = lines[i].trim();

                    // Check if this is a single DISCONTINUITY marking end of ad
                    if current.contains("#EXT-X-DISCONTINUITY") {
                        // Skip this DISCONTINUITY and continue with content
                        i += 1;
                        break;
                    }

                    // Skip EXTINF line
                    if current.starts_with("#EXTINF:") {
                        i += 1;
                        continue;
                    }

                    // Skip non-empty, non-comment lines (these are segment URLs)
                    if !current.is_empty() && !current.starts_with('#') {
                        i += 1;
                        continue;
                    }

                    // Skip other tags/empty lines
                    i += 1;
                }
                continue;
            }

            if line.starts_with("#EXTINF:") {
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
                    if Self::is_ad_url(url) {
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
    fn filters_ad_segments_by_domain() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://content-cdn.example.com/seg1.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // ad-cdn.example.com contains "/ad/" in path - should be filtered
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        // Content segment should be preserved
        assert!(result.contains("content-cdn.example.com/seg1.ts"));
    }

    #[test]
    fn passes_through_clean_playlist_unchanged() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn removes_discontinuity_before_ad_segment() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:15.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        assert!(!result.contains("#EXT-X-DISCONTINUITY"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
    }

    #[test]
    fn handles_master_playlist_with_variant_urls() {
        let playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nhttps://cdn.example.com/variant.m3u8\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn filters_ad_segments_by_url_pattern() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/ad-1001.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(!result.contains("ad-1001.ts"));
        assert!(result.contains("seg1.ts"));
    }

    #[test]
    fn handles_empty_playlist() {
        let result = HlsAdBlocker::filter_playlist("");
        assert_eq!(result, "");
    }

    #[test]
    fn handles_playlist_with_only_ads() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(result.contains("#EXTM3U"));
        assert!(!result.contains("#EXTINF"));
    }

    #[test]
    fn filters_ad_segments_by_double_discontinuity() {
        // Double DISCONTINUITY marks ad break start
        let playlist = "#EXTM3U\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad_seg1.ts\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad_seg2.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // Content segments should be preserved
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
        // Ad segments should be removed
        assert!(!result.contains("ad_seg1.ts"));
        assert!(!result.contains("ad_seg2.ts"));
        // No double DISCONTINUITY in result
        assert!(!result.contains("#EXT-X-DISCONTINUITY\n#EXT-X-DISCONTINUITY"));
    }

    #[test]
    fn double_discontinuity_removes_leading_discontinuity() {
        // The single DISCONTINUITY before double DISCONTINUITY should be removed
        // as it was joining content to ad
        let playlist = "#EXTM3U\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad_seg1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(!result.contains("#EXT-X-DISCONTINUITY"));
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
    }

    #[test]
    fn double_discontinuity_multiple_ad_breaks() {
        let playlist = "#EXTM3U\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad2.ts\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad3.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg3.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // Content preserved
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(result.contains("cdn.example.com/seg2.ts"));
        assert!(result.contains("cdn.example.com/seg3.ts"));
        // Ads removed
        assert!(!result.contains("ad1.ts"));
        assert!(!result.contains("ad2.ts"));
        assert!(!result.contains("ad3.ts"));
    }

    #[test]
    fn double_discontinuity_at_playlist_start() {
        let playlist = "#EXTM3U\n\
#EXT-X-DISCONTINUITY\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/ad_seg1.ts\n\
#EXT-X-DISCONTINUITY\n\
#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(!result.contains("ad_seg1.ts"));
    }
}
