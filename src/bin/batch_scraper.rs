// Project: justice-scraper
// File: src/bin/batch_scraper.rs
// Description: Resume financial document retrieval from PO_ALL.csv.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
#[tokio::main]
async fn main() {
    if justice_scraper::jobs::cli_help(
        "batch_scraper",
        "Read PO_ALL.csv and append financials_links.csv, skipping processed companies.",
    ) {
        return;
    }
    let result = async {
        justice_scraper::http::init_batch_log()?;
        justice_scraper::jobs::batch(
            std::path::Path::new("PO_ALL.csv"),
            std::path::Path::new("financials_links.csv"),
        )
        .await
    }
    .await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
