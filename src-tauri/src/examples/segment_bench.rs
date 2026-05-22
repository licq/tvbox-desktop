//! Benchmark tool for HTTP segment fetching
//! Run with: cargo run --example segment_bench

use std::time::Instant;

#[tokio::main]
async fn main() {
    let segment_url = "https://yyv2.qwe132456.cc/yyv2/202604/24/ms3vFU6k2624/video/2000k_1080/hls/Vq7UglVWB5.ts";
    
    // Test 1: HTTP/1.1 (current implementation)
    println!("=== Test 1: HTTP/1.1 ===");
    let client_h1 = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http1_only()
        .pool_max_idle_per_host(16)
        .build()
        .unwrap();

    let start = Instant::now();
    match client_h1.get(segment_url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .send()
        .await
    {
        Ok(r) => {
            let elapsed = start.elapsed();
            let status = r.status();
            println!("Status: {}, Time: {:.3}s", status, elapsed.as_secs_f64());
        }
        Err(e) => println!("Failed: {}", e),
    }
    
    // Test 2: HTTP/2 with prior knowledge
    println!("\n=== Test 2: HTTP/2 (prior knowledge) ===");
    let client_h2 = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http2_prior_knowledge()
        .pool_max_idle_per_host(16)
        .build()
        .unwrap();
    
    let start = Instant::now();
    match client_h2.get(segment_url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .send()
        .await
    {
        Ok(r) => {
            let elapsed = start.elapsed();
            let status = r.status();
            println!("Status: {}, Time: {:.3}s", status, elapsed.as_secs_f64());
        }
        Err(e) => println!("Failed: {}", e),
    }
    
    // Test 3: HTTP/2 with ALPN negotiation
    println!("\n=== Test 3: HTTP/2 (ALPN negotiation) ===");
    let client_h2_auto = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(16)
        .build()
        .unwrap();
    
    let start = Instant::now();
    match client_h2_auto.get(segment_url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .send()
        .await
    {
        Ok(r) => {
            let elapsed = start.elapsed();
            let status = r.status();
            println!("Status: {}, Time: {:.3}s", status, elapsed.as_secs_f64());
        }
        Err(e) => println!("Failed: {}", e),
    }
    
    // Test 4: Test 3 consecutive fetches to see connection reuse
    println!("\n=== Test 4: Connection reuse (3 consecutive fetches) ===");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http1_only()
        .pool_max_idle_per_host(16)
        .build()
        .unwrap();
    
    for i in 0..3 {
        let start = Instant::now();
        match client.get(segment_url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .send()
            .await
        {
            Ok(r) => {
                let elapsed = start.elapsed();
                println!("Fetch {}: {:.3}s, status: {}", i+1, elapsed.as_secs_f64(), r.status());
            }
            Err(e) => println!("Fetch {} failed: {}", i+1, e),
        }
    }
}