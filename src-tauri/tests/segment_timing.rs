// Segment timing benchmark - measures HLS segment fetch performance
// Run with: cargo test --test segment_timing -- --nocapture

use std::time::Instant;

#[tokio::test]
async fn segment_fetch_timing() {
    use base64::Engine;
    
    let segment_url = "https://yyv2.qwe132456.cc/yyv2/202604/24/ms3vFU6k2624/video/2000k_1080/hls/Vq7UglVWB5.ts";
    
    println!("\n=== Testing different client configurations ===");
    
    // Test 1: Current configuration (HTTP/1.1 with pooling)
    let client_h1 = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http1_only()
        .pool_max_idle_per_host(16)
        .build()
        .unwrap();
    
    let start = Instant::now();
    match client_h1.get(segment_url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .send()
        .await
    {
        Ok(r) => {
            let fetch_time = start.elapsed();
            let bytes_start = Instant::now();
            let bytes = r.bytes().await.unwrap();
            let read_time = bytes_start.elapsed();
            println!("  HTTP/1.1 (pooled): fetch={:.3}s, read={:.3}s, total={:.3}s, size={}",
                fetch_time.as_secs_f64(), read_time.as_secs_f64(), start.elapsed().as_secs_f64(), bytes.len());
        }
        Err(e) => println!("  Failed: {}", e),
    }
    
    // Test 2: HTTP/1.1 without pooling
    let client_no_pool = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http1_only()
        .pool_max_idle_per_host(0)
        .build()
        .unwrap();
    
    let start = Instant::now();
    match client_no_pool.get(segment_url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .send()
        .await
    {
        Ok(r) => {
            let fetch_time = start.elapsed();
            let bytes_start = Instant::now();
            let bytes = r.bytes().await.unwrap();
            let read_time = bytes_start.elapsed();
            println!("  HTTP/1.1 (no pool): fetch={:.3}s, read={:.3}s, total={:.3}s, size={}",
                fetch_time.as_secs_f64(), read_time.as_secs_f64(), start.elapsed().as_secs_f64(), bytes.len());
        }
        Err(e) => println!("  Failed: {}", e),
    }
    
    // Test 3: Try with hyper client (hyper-based for comparison)
    println!("\n=== Testing connection reuse ===");
    for i in 0..3 {
        let start = Instant::now();
        match client_h1.get(segment_url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .send()
            .await
        {
            Ok(r) => {
                let fetch_time = start.elapsed();
                let bytes = r.bytes().await.unwrap();
                let read_time = start.elapsed().checked_sub(fetch_time).unwrap_or_default();
                println!("  Request {}: fetch={:.3}s, read={:.3}s, total={:.3}s",
                    i + 1, fetch_time.as_secs_f64(), read_time.as_secs_f64(), start.elapsed().as_secs_f64());
            }
            Err(e) => println!("  Failed: {}", e),
        }
    }
    
    println!("\nConclusion: If 'read' time is high, it indicates server-side rate limiting.");
    println!("If 'fetch' time is high, it indicates connection setup issues.");
}
