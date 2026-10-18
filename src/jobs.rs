// Project: justice-scraper
// File: src/jobs.rs
// Description: Concurrent CSV scraping and document downloads.
// Author: Jan Alexandr Kopřiva
// License: MIT
use crate::{
    html::*, http::*, parsers::extract_pdf_text, service::fetch_justice_financials, Error,
};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Semaphore, task::JoinSet};
pub fn writer<W: std::io::Write>(w: W) -> csv::Writer<W> {
    csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(w)
}
async fn range_fetch(client: &reqwest::Client, url: &str) -> Option<String> {
    for (attempt, delay) in [1, 2, 4].into_iter().enumerate() {
        match client
            .get(url)
            .header("User-Agent", RANGE_AGENT)
            .header("Accept", ACCEPT)
            .header("Accept-Language", "cs,en;q=0.8")
            .header("Referer", format!("{ORIGIN}/ias/ui/"))
            .send()
            .await
        {
            Ok(r) => {
                if r.status().as_u16() == 200 {
                    if let Ok(s) = r.text().await {
                        return Some(s);
                    }
                } else {
                    log(
                        "WARNING",
                        &format!(
                            "[{url}] HTTP {} (attempt {}/3)",
                            r.status().as_u16(),
                            attempt + 1
                        ),
                    );
                }
            }
            Err(e) => log(
                "WARNING",
                &format!(
                    "[{url}] {} (attempt {}/3)",
                    if e.is_timeout() {
                        "TimeoutError"
                    } else {
                        "ClientError"
                    },
                    attempt + 1
                ),
            ),
        }
        tokio::time::sleep(Duration::from_secs(delay)).await;
    }
    None
}
pub async fn scrape_range(
    start: u64,
    end: u64,
    output: &Path,
    origin: &str,
) -> Result<usize, Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .cookie_store(true)
        .timeout(Duration::from_secs(20))
        .build()?;
    let sem = Arc::new(Semaphore::new(20));
    let mut tasks = JoinSet::new();
    for id in start..=end {
        let client = client.clone();
        let sem = sem.clone();
        let origin = origin.to_owned();
        tasks.spawn(async move {
            let _permit = sem.acquire_owned().await.ok()?;
            let main = range_fetch(
                &client,
                &format!("{origin}/ias/ui/vypis-sl-firma?subjektId={id:06}"),
            )
            .await?;
            let (ico, detail, date) = parse_main_page(&main);
            let ico = ico?;
            let detail = detail?;
            let detail = detail.replacen(ORIGIN, &origin, 1);
            let html = range_fetch(&client, &detail).await?;
            let (url, kind) = parse_document_link(&html)?;
            Some([ico, date.unwrap_or_default(), url, kind])
        });
    }
    let mut csv = writer(fs::File::create(output)?);
    csv.write_record(["ico", "vznik_listiny", "link", "typ"])?;
    let mut count = 0;
    while let Some(row) = tasks.join_next().await {
        match row {
            Ok(Some(row)) => {
                csv.write_record(row)?;
                count += 1;
            }
            Err(e) => log("ERROR", &format!("Task failed: {e}")),
            _ => (),
        }
    }
    csv.flush()?;
    log(
        "INFO",
        &format!("Finished: saved {count} rows to {}", output.display()),
    );
    Ok(count)
}
fn read_rows(path: &Path) -> Result<Vec<csv::StringRecord>, Error> {
    let raw = fs::read(path)?;
    // Python opens input CSVs with universal newline translation, including
    // carriage returns inside quoted fields.
    let mut bytes = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'\r' {
            bytes.push(b'\n');
            index += if raw.get(index + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        } else {
            bytes.push(raw[index]);
            index += 1;
        }
    }
    let mut rows = vec![];
    let mut prefix = bytes.as_slice();
    // The CSV crate skips blank lines; Python consumes a blank first header.
    while prefix.starts_with(b"\r") || prefix.starts_with(b"\n") {
        let skip = if prefix.starts_with(b"\r\n") { 2 } else { 1 };
        rows.push(csv::StringRecord::new());
        prefix = &prefix[skip..];
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes.as_slice());
    rows.extend(reader.records().collect::<Result<Vec<_>, _>>()?);
    Ok(rows)
}
pub async fn batch(input: &Path, output: &Path) -> Result<(), Error> {
    let processed: HashSet<String> = if output.exists() {
        read_rows(output)?
            .into_iter()
            .skip(1)
            .filter_map(|r| r.get(0).map(str::to_owned))
            .collect()
    } else {
        HashSet::new()
    };
    let icos: Vec<_> = read_rows(input)?
        .into_iter()
        .skip(1)
        .filter_map(|r| {
            r.get(0)
                .map(|s| s.trim_matches(crate::parsers::python_whitespace).to_owned())
        })
        .filter(|i| !processed.contains(i))
        .collect();
    println!(
        "Total entries found. Already processed: {}. Remaining: {}",
        processed.len(),
        icos.len()
    );
    if icos.is_empty() {
        return Ok(());
    }
    let exists = output.exists();
    let mut csv = writer(OpenOptions::new().create(true).append(true).open(output)?);
    if !exists {
        csv.write_record([
            "ico",
            "subjektId",
            "dokumentId",
            "year",
            "documentUrl",
            "sourceType",
            "parserUsed",
            "sourceNotes",
        ])?;
        csv.flush()?;
    }
    let sem = Arc::new(Semaphore::new(10));
    let mut tasks = JoinSet::new();
    for ico in icos {
        let sem = sem.clone();
        tasks.spawn(async move {
            let _permit = sem.acquire_owned().await?;
            fetch_justice_financials(&ico).await
        });
    }
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(v)) => {
                let value = |v: &serde_json::Value| {
                    if v.is_null() {
                        "".into()
                    } else if let Some(s) = v.as_str() {
                        s.into()
                    } else {
                        v.to_string()
                    }
                };
                let records = v["financials"].as_array().unwrap();
                if records.is_empty() {
                    csv.write_record([
                        value(&v["ico"]),
                        value(&v["subjektId"]),
                        "".into(),
                        "".into(),
                        "".into(),
                        "".into(),
                        "".into(),
                        "no_records".into(),
                    ])?;
                } else {
                    for f in records {
                        csv.write_record([
                            value(&v["ico"]),
                            value(&v["subjektId"]),
                            value(&f["dokumentId"]),
                            value(&f["year"]),
                            value(&f["documentUrl"]),
                            value(&f["sourceType"]),
                            value(&f["parserUsed"]),
                            f["sourceNotes"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v.as_str().unwrap_or(""))
                                .collect::<Vec<_>>()
                                .join("|"),
                        ])?;
                    }
                }
                csv.flush()?;
            }
            Ok(Err(e)) => log("ERROR", &format!("Error processing ICO: {e}")),
            Err(e) => log("ERROR", &format!("Error processing ICO: {e}")),
        }
    }
    Ok(())
}
pub async fn download(input: &Path, dir: &Path, origin: &str) -> Result<(), Error> {
    fs::create_dir_all(dir)?;
    if !input.exists() {
        println!(
            "File {} not found. Run batch_scraper first.",
            input.display()
        );
        return Ok(());
    }
    let rows = read_rows(input)?;
    if rows.first().is_none_or(|row| row.is_empty()) {
        return Ok(());
    }
    let rows = rows
        .into_iter()
        .skip(1)
        .filter(|r| r.len() >= 4 && !r[1].is_empty() && !r[2].is_empty())
        .collect::<Vec<_>>();
    println!("Found {} documents to download.", rows.len());
    let queue = Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::from(
        rows,
    )));
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let queue = queue.clone();
        let dir = dir.to_owned();
        let origin = origin.to_owned();
        tasks.spawn(async move {
            let client = reqwest::Client::builder()
                .no_proxy()
                .cookie_store(true)
                .timeout(Duration::from_secs(30))
                .user_agent(USER_AGENTS[0])
                .build()?;
            loop {
                let Some(row) = queue.lock().await.pop_front() else {
                    break;
                };
                if let Err(e) = download_one(&client, &row, &dir, &origin).await {
                    log(
                        "ERROR",
                        &format!("{}: Exception during download: {e}", &row[0]),
                    );
                }
            }
            Ok::<(), Error>(())
        });
    }
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(())) => (),
            Ok(Err(e)) => log("ERROR", &format!("Exception during download: {e}")),
            Err(e) => log("ERROR", &format!("Exception during download: {e}")),
        }
    }
    Ok(())
}
async fn download_one(
    client: &reqwest::Client,
    row: &csv::StringRecord,
    dir: &Path,
    origin: &str,
) -> Result<(), Error> {
    let (ico, id, doc, year) = (&row[0], &row[1], &row[2], &row[3]);
    if !doc.chars().all(|c| c.is_ascii_digit()) {
        log(
            "ERROR",
            &format!("{ico}: Invalid dokument_id '{doc}', skipping"),
        );
        return Ok(());
    }
    // Every filename component must be safe, not only the document identifier.
    if [ico, year]
        .iter()
        .any(|s| s.contains(['/', '\\']) || *s == ".." || s.contains(':'))
    {
        log(
            "ERROR",
            &format!("{ico}: Invalid output filename, skipping"),
        );
        return Ok(());
    }
    let path = dir.join(format!("{ico}_{year}_{doc}.txt"));
    if path.exists() {
        return Ok(());
    }
    let list_url = format!("{origin}/ias/ui/vypis-sl-firma?subjektId={id}");
    let r = client
        .get(&list_url)
        .header("Accept", ACCEPT)
        .send()
        .await?;
    if r.status().as_u16() != 200 {
        log(
            "ERROR",
            &format!(
                "{ico}: Failed to load listing page (HTTP {})",
                r.status().as_u16()
            ),
        );
        return Ok(());
    }
    let listing = r.text().await?;
    let listing = HtmlDoc::new(&listing);
    let href = listing
        .links
        .iter()
        .find(|h| h.contains(&format!("dokument={doc}")));
    let Some(href) = href else {
        log(
            "ERROR",
            &format!("{ico}: Dokument {doc} not found on listing page."),
        );
        return Ok(());
    };
    let detail_url = join(&list_url, href);
    let r = client
        .get(&detail_url)
        .header("Accept", ACCEPT)
        .header("Referer", &list_url)
        .send()
        .await?;
    if r.status().as_u16() != 200 {
        log(
            "ERROR",
            &format!(
                "{ico}: Failed to load detail page (HTTP {})",
                r.status().as_u16()
            ),
        );
        return Ok(());
    }
    let detail = r.text().await?;
    if detail.contains("Neplatný odkaz") {
        log(
            "ERROR",
            &format!("{ico}: Detail page returned 'Invalid link' despite session flow."),
        );
        return Ok(());
    }
    let detail = HtmlDoc::new(&detail);
    let Some(href) = detail
        .links
        .iter()
        .find(|h| h.contains("download") && !h.contains("GDPR"))
    else {
        log(
            "ERROR",
            &format!("{ico}: No download link found on detail page."),
        );
        return Ok(());
    };
    let r = client
        .get(join(&detail_url, href))
        .header("Accept", ACCEPT)
        .header("Referer", &detail_url)
        .send()
        .await?;
    if r.status().as_u16() != 200 {
        log(
            "ERROR",
            &format!(
                "{ico}: Failed to download file (HTTP {})",
                r.status().as_u16()
            ),
        );
        return Ok(());
    }
    if r.headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .contains("text/html")
    {
        log(
            "ERROR",
            &format!("{ico}: Download returned HTML instead of a file."),
        );
        return Ok(());
    }
    let bytes = r.bytes().await?;
    match tokio::task::spawn_blocking(move || extract_pdf_text(&bytes, None)).await? {
        Ok(text) => {
            #[cfg(windows)]
            let text = text.replace('\n', "\r\n");
            fs::write(path, text)?;
        }
        Err(_) => log("ERROR", &format!("{ico}: Failed to extract text from PDF")),
    };
    Ok(())
}
struct HtmlDoc {
    links: Vec<String>,
}
impl HtmlDoc {
    fn new(s: &str) -> Self {
        let doc = scraper::Html::parse_document(s);
        Self {
            links: doc
                .select(&select("a[href]"))
                .filter_map(|a| a.value().attr("href").map(str::to_owned))
                .collect(),
        }
    }
}
pub fn cli_help(name: &str, description: &str) -> bool {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!("{description}\n\nUsage: {name}\n\nOptions:\n  -h, --help  Print help\n\nRun from the directory containing the input CSV files. No configuration or environment variables are required. PDF OCR in the library needs pdftoppm and tesseract (ces+eng) on PATH.");
        true
    } else {
        false
    }
}
