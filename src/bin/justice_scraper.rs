// Project: justice-scraper
// File: src/bin/justice_scraper.rs
// Description: Scrape subject IDs 1 through 1000.
// Author: Jan Alexandr Kopřiva
// License: MIT
#[tokio::main]
async fn main() {
    if justice_scraper::jobs::cli_help(
        "justice_scraper",
        "Scrape subject IDs 1 through 1000 into document_links.csv.",
    ) {
        return;
    }
    let start = std::time::Instant::now();
    let result = async {
        justice_scraper::http::init_log("scraper.log", false, true)?;
        justice_scraper::jobs::scrape_range(
            1,
            1000,
            std::path::Path::new("document_links.csv"),
            justice_scraper::html::ORIGIN,
        )
        .await?;
        justice_scraper::http::log(
            "INFO",
            &format!("Runtime: {:.2}s", start.elapsed().as_secs_f64()),
        );
        Ok::<(), justice_scraper::Error>(())
    }
    .await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
