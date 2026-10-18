# justice-scraper

Pulls financial statement links and figures from the Czech Justice Registry.

[![CI](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)](LICENSE)

The registry has no public API. This Rust application follows its HTML pages and attached documents to collect statement links and accounting figures. It is a prototype: keyword matching can select incorrect amounts, so use the results as leads requiring verification. Records with internally contradictory figures carry low confidence and explanatory source notes.

## What it does

- Walks subject IDs 1 through 1000 and writes `document_links.csv`.
- Reads company identifiers from the first column of `PO_ALL.csv`, after its header, and appends results to `financials_links.csv`.
- Resumes batch processing by skipping identifiers already present in the output.
- Downloads listed documents and writes extracted PDF text into `texts/`.
- Exposes an asynchronous Rust library for retrieving one company's financial records, preferring XML, then XHTML, then PDF links.

## Install

Install a current stable Rust toolchain, then run:

```sh
cargo install --path .
```

Optional library OCR requires Poppler's `pdftoppm` and Tesseract with Czech and English language data available on `PATH`.

## Use

Run commands from the directory containing your input CSV files:

```sh
justice_scraper
batch_scraper
download_pdfs
```

The batch input must contain a header and company identifiers in its first column. Run batch processing before downloading. Existing text files are skipped. See each executable's `--help` for its purpose and default files. For library integration, call `justice_scraper::fetch_justice_financials` from a Tokio runtime; it returns the record structure as a JSON value.

## License

[MIT](LICENSE). Copyright © 2025 Jan Alexandr Kopřiva.
