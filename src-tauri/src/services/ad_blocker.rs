/// HLS ad segment filtering.
///
/// Parses HLS playlists (m3u8) and removes segments whose URLs
/// match known ad CDN domains or URL patterns.
pub struct HlsAdBlocker;

/// Known ad CDN domain fragments and URL path patterns.
const AD_PATTERNS: &[&str] = &[
    // URL path patterns
    "/ad/",
    "ad-",
    ".ad.",
    "adservice",
    "-ads-",
    "adtrack",
    "doubleclick",
    "googlesyndication",
];

impl HlsAdBlocker {
    /// Check if a URL belongs to a known ad CDN or matches an ad pattern.
    fn is_ad_url(url: &str) -> bool {
        let url_lower = url.to_lowercase();
        AD_PATTERNS
            .iter()
            .any(|&pattern| url_lower.contains(pattern))
    }

    /// Remove ad segments from an HLS playlist.
    ///
    /// Scans the playlist for `#EXTINF:` + URL pairs. When a segment URL matches
    /// the ad blacklist, both the `#EXTINF:` line and the URL line are removed.
    /// Any `#EXT-X-DISCONTINUITY` line immediately preceding a removed ad segment
    /// is also removed.
    ///
    /// Master playlists (containing `#EXT-X-STREAM-INF`) are passed through
    /// unchanged -- they contain variant URLs, not segment URLs.
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
}
