use std::sync::Arc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use indicatif::ProgressBar;
use reqwest::Client;
use tokio::sync::{Mutex, Semaphore};

#[tokio::main]
async fn main() {
    println!("Starting stress test...");

    let url = "https://www.yourwebsite.com";
    let total_requests = 100_000;
    let concurrency = 500;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let histogram = Arc::new(Mutex::new(Histogram::<u64>::new(3).unwrap()));

    let progress_bar = ProgressBar::new(total_requests as u64);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    // progress_bar.set_message("");

    let mut handles = Vec::with_capacity(total_requests);

    let start = Instant::now();

    for _ in 0..total_requests {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.to_string();
        let histogram = histogram.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            let t0 = Instant::now();

            let result = client.get(&url).send().await;

            let elapsed = t0.elapsed().as_micros() as u64;

            if result.is_ok() {
                let mut h = histogram.lock().await;
                let _ = h.record(elapsed);
            }
        });

        progress_bar.inc(1);
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    progress_bar.finish();
    let elapsed = start.elapsed();
    let histogram = histogram.lock().await;

    let total_secs = elapsed.as_secs_f64();
    let count = histogram.len() as f64;

    println!("--- Stress Test Results ---");
    println!("Total requests: {}", total_requests);
    println!("Completed: {}", histogram.len());
    println!("Elapsed: {:.2}s", total_secs);
    println!("Throughput: {:.2} req/s", count / total_secs);
    println!("Avg latency: {:.2} ms", histogram.mean() / 1000.0);
    println!(
        "P95 latency: {:.2} ms",
        histogram.value_at_quantile(0.95) as f64 / 1000.0
    );
    println!(
        "P99 latency: {:.2} ms",
        histogram.value_at_quantile(0.99) as f64 / 1000.0
    );
}
