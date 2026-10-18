// Project: justice-scraper
// File: src/http.rs
// Description: Session HTTP policies and Python-style logging.
// Author: Jan Alexandr Kopřiva
// License: MIT
use std::{
    io::Write,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::Mutex as AsyncMutex;
pub const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
pub const USER_AGENTS: [&str;5]=[
"Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
"Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0",
"Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:126.0) Gecko/20100101 Firefox/126.0",
"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0"];
pub const RANGE_AGENT: &str="Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Safari/605.1.15";
static LOGGER: Mutex<Option<(std::fs::File, bool)>> = Mutex::new(None);
pub fn init_log(path: &str, append: bool, console: bool) -> Result<(), crate::Error> {
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)?;
    *LOGGER.lock().unwrap() = Some((f, console));
    Ok(())
}
pub fn log(level: &str, msg: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S,%3f");
    let mut logger = LOGGER.lock().unwrap();
    if let Some((f, console)) = logger.as_mut() {
        if !*console && level == "INFO" {
            return;
        }
        let line = format!("{now} {level:>8} | {msg}\n");
        let _ = f.write_all(line.as_bytes());
        if *console {
            eprint!("{line}");
        }
    } else {
        eprintln!("{now} {level} justice_fetcher | {msg}");
    }
}
pub struct RateLimiter {
    last: AsyncMutex<Option<Instant>>,
    interval: Duration,
}
impl RateLimiter {
    pub fn new(seconds: f64) -> Self {
        Self {
            last: AsyncMutex::new(None),
            interval: Duration::from_secs_f64(seconds),
        }
    }
    pub async fn wait_turn(&self) {
        let mut last = self.last.lock().await;
        if let Some(t) = *last {
            tokio::time::sleep(self.interval.saturating_sub(t.elapsed())).await;
        }
        *last = Some(Instant::now());
    }
}
pub struct JusticeHttpClient {
    pub session: reqwest::Client,
    pub origin: String,
    limiter: RateLimiter,
}
impl JusticeHttpClient {
    pub fn new(origin: &str) -> Result<Self, crate::Error> {
        Ok(Self {
            session: reqwest::Client::builder()
                .no_proxy()
                .cookie_store(true)
                .timeout(Duration::from_secs(8))
                .default_headers({
                    let mut h = reqwest::header::HeaderMap::new();
                    h.insert("Accept", ACCEPT.parse()?);
                    h.insert("Accept-Language", "cs,en;q=0.8".parse()?);
                    h.insert("Connection", "keep-alive".parse()?);
                    h
                })
                .build()?,
            origin: origin.trim_end_matches('/').into(),
            limiter: RateLimiter::new(0.05),
        })
    }
    pub async fn fetch(
        &self,
        url: &str,
        ico: &str,
        step: &str,
        referer: Option<&str>,
        bytes: bool,
    ) -> Option<Vec<u8>> {
        let attachment = ["step3_xml_", "step3_xhtml_", "step3_pdf_"]
            .iter()
            .any(|p| step.starts_with(p));
        let retries = usize::from(!attachment);
        let agent = USER_AGENTS[rand::random_range(0..USER_AGENTS.len())];
        let default_ref = format!("{}/ias/ui/", self.origin);
        for attempt in 0..=retries {
            if !attachment {
                self.limiter.wait_turn().await;
            }
            let result = self
                .session
                .get(url)
                .header("User-Agent", agent)
                .header("Referer", referer.unwrap_or(&default_ref))
                .timeout(Duration::from_secs(if attachment { 12 } else { 8 }))
                .send()
                .await;
            let error = match result {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success() {
                        let content_type = r
                            .headers()
                            .get("Content-Type")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string();
                        if bytes {
                            match r.bytes().await {
                                Ok(b) => {
                                    if b.starts_with(b"<!DOC")
                                        || b.starts_with(b"<html")
                                        || b.starts_with(b"<HTML")
                                        || content_type.contains("text/html")
                                    {
                                        log("INFO",&format!("justice-scraper ico={ico} status=html_instead_of_file step={step}"));
                                        return None;
                                    }
                                    return Some(b.to_vec());
                                }
                                Err(e) => Some(e),
                            }
                        } else {
                            match r.text().await {
                                Ok(s) => return Some(s.into_bytes()),
                                Err(e) => Some(e),
                            }
                        }
                    } else {
                        log(
                            "INFO",
                            &format!(
                                "justice-scraper ico={ico} status=http_{} step={step} attempt={}",
                                status.as_u16(),
                                attempt + 1
                            ),
                        );
                        if !status.is_server_error() || attempt == retries {
                            return None;
                        }
                        None
                    }
                }
                Err(e) => Some(e),
            };
            if let Some(e) = error {
                log(
                    "INFO",
                    &format!(
                        "justice-scraper ico={ico} status=error step={step} error={} attempt={}",
                        if e.is_timeout() {
                            "TimeoutError"
                        } else {
                            "ClientError"
                        },
                        attempt + 1
                    ),
                );
                if attempt == retries {
                    return None;
                }
            }
            if attempt < retries {
                tokio::time::sleep(Duration::from_millis(200 * (attempt as u64 + 1))).await;
            }
        }
        None
    }
    pub async fn text(
        &self,
        url: &str,
        ico: &str,
        step: &str,
        referer: Option<&str>,
    ) -> Option<String> {
        self.fetch(url, ico, step, referer, false)
            .await
            .and_then(|b| String::from_utf8(b).ok())
    }
}

// Importing the original library installed the console handler before the batch script.
pub fn init_batch_log() -> Result<(), crate::Error> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("batch_scraper.log")?;
    *LOGGER.lock().unwrap() = None;
    Ok(())
}
