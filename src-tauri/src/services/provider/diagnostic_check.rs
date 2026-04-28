/// Targeted domain discovery: probe alternative domains for 404/dead sources
/// and test if they return actual video content.
#[tokio::test]
async fn domain_discovery_probe() {
    use std::time::Duration;
    let timeout = Duration::from_secs(10);
    let _keyword = "功夫";

    #[derive(Debug)]
    struct ProbeResult {
        domain: &'static str,
        status: u16,
        body_len: usize,
        note: &'static str,
    }

    // All domains to probe
    let targets = vec![
        // Previously found potentially live domains
        ("ddrk", "https://ddrk.com/", "新域名"),
        ("ddrd", "https://www.ddrd.com/", "新域名"),
        ("auete_com", "https://auete.com/", "可能是新站"),
        ("aote", "https://www.aote.com/", "可能是新站"),
        ("miso", "https://www.miso.com/", "Framer网站，非视频站"),
        ("dm84", "https://dm84.com/", "重定向目标待查"),
        ("cc", "https://cc.com/", "404待重探"),

        // More alternatives for dead sources
        ("zxzj_alt1", "https://zxzj.com/", "在线备选"),
        ("zxzj_alt2", "https://www.zxzj.net/", "在线备选2"),
        ("yicai_alt", "https://yicai.tv/", "溢彩备选"),
        ("mengmi_alt", "https://www.mengmi.tv/", "萌米备选"),
        ("libvio_alt1", "https://libvio.tv/", "立播备选1"),
        ("libvio_alt2", "https://www.libvio.net/", "立播备选2"),
        ("auete_alt1", "https://www.aoete.com/", "奥特备选1"),
        ("auete_alt2", "https://www.autey.com/", "奥特备选2"),
        ("auete_alt3", "https://www.aote.net/", "奥特备选3"),

        // nuomi alternatives
        ("nuomi_alt1", "https://www.nuomiv.com/", "糯米备选1"),
        ("nuomi_alt2", "https://nuomip.com/", "糯米备选2"),
        ("nuomi_alt3", "https://nuomipp.com/", "糯米备选3"),
        ("nuomi_alt4", "https://www.nuomipp.com/", "糯米备选4"),
        ("nuomi_alt5", "https://www.nuomi.net/", "糯米备选5"),

        // wencai alternatives
        ("wencai_alt1", "https://www.wencai.tv/", "文采备选1"),
        ("wencai_alt2", "https://www.wencai.net/", "文采备选2"),
        ("wencai_alt3", "https://www.wencx.com/", "文采备选3"),
        ("wencai_alt4", "https://www.wencai.co/", "文采备选4"),

        // yuanchuang alternatives
        ("yuanchuang_alt1", "https://www.yuanchuang.tv/", "原创备选1"),
        ("yuanchuang_alt2", "https://www.yuanchuang.net/", "原创备选2"),
        ("yuanchuang_alt3", "https://www.yuanchuang.cc/", "原创备选3"),

        // chz alternatives
        ("chz_alt1", "https://changzhang.com/", "厂长备选1"),
        ("chz_alt2", "https://www.changzhang.com/", "厂长备选2"),
        ("chz_alt3", "https://www.cz8.com/", "厂长备选3"),
        ("chz_alt4", "https://cz8.com/", "厂长备选4"),
    ];

    let mut results: Vec<ProbeResult> = Vec::new();

    for (name, url, note) in targets {
        let resp = tokio::time::timeout(timeout, reqwest::get(url)).await;
        match resp {
            Ok(Ok(r)) => {
                let status = r.status().as_u16();
                let body = r.text().await.unwrap_or_default();
                let body_len = body.len();
                let has_cms = body.contains("maccms") || body.contains("Mac") || body.contains("cms")
                    || body.contains("stui") || body.contains("stui_default") || body.contains("vod");
                let has_wp = body.contains("wordpress") || body.contains("wp-content");
                let is_landing = body_len < 200 || body.contains("lander") || body.contains("parked") || body.contains("godaddy") || body.contains("sale");
                let is_framer = body.contains("framer") || body.contains("Framer");

                let result_note = if status >= 400 {
                    "http_error"
                } else if is_landing {
                    "landed/parked"
                } else if is_framer {
                    "framer_site"
                } else if has_wp {
                    "wordpress"
                } else if has_cms {
                    "cms_site"
                } else if body_len < 500 {
                    "small_page"
                } else {
                    "interesting"
                };

                results.push(ProbeResult { domain: name, status, body_len, note });
                eprintln!("[PROBE] {:<20} {:<8} {:<10} {:<20} {} ({})",
                    name, status, body_len, result_note, note, if has_cms { "CMS!" } else { "" });
            }
            Ok(Err(_)) => {
                results.push(ProbeResult { domain: name, status: 0, body_len: 0, note });
                eprintln!("[PROBE] {:<20} CONN_ERR  {:<10} {}", name, note, "");
            }
            Err(_) => {
                results.push(ProbeResult { domain: name, status: 998, body_len: 0, note });
                eprintln!("[PROBE] {:<20} TIMEOUT  {:<10} {}", name, note, "");
            }
        }
    }

    eprintln!("\n========== DOMAIN DISCOVERY SUMMARY ==========");
    for r in &results {
        if r.status >= 200 && r.status < 400 && r.note != "landed/parked" && r.note != "wordpress" {
            eprintln!("  INTERESTING: {} ({}) - status={} len={}", r.domain, r.note, r.status, r.body_len);
        }
    }
}
