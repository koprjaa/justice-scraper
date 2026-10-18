// Project: justice-scraper
// File: src/bin/download_pdfs.rs
// Description: Download financial documents and extract PDF text.
// Author: Jan Alexandr Kopřiva
// License: MIT
#[tokio::main]
async fn main() {
    if justice_scraper::jobs::cli_help(
        "download_pdfs",
        "Read financials_links.csv and extract PDFs into texts/. Existing files are skipped.",
    ) {
        return;
    }
    let result = async {
        justice_scraper::http::init_log("pdf_downloader.log", true, false)?;
        justice_scraper::jobs::download(
            std::path::Path::new("financials_links.csv"),
            std::path::Path::new("texts"),
            justice_scraper::html::ORIGIN,
        )
        .await
    }
    .await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
