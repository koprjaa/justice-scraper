# justice-scraper

Pulls financial statement links and figures from the Czech Justice Registry.

[![CI](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-AGPL--3.0-A31F34?style=flat-square)](LICENSE)

## What it does

- `justice_scraper` scans subject IDs 1 through 1000 and overwrites `document_links.csv` with statement links.
- `batch_scraper` reads company identifiers (IČO) from `PO_ALL.csv` and appends document links and metadata to `financials_links.csv`. It skips identifiers already in the output.
- `download_pdfs` reads the batch output and saves extracted PDF text as `texts/<ico>_<year>_<document-id>.txt`. It skips existing text files.

The Rust library's `fetch_justice_financials` returns JSON with financial records for one company. It prefers XML, then XHTML, and collects PDF links without parsing their figures. Batch CSV output contains links and metadata; financial amounts remain in the library result. Keyword matching can select incorrect amounts, so check extracted figures against their source documents.

## Install

Install a stable Rust toolchain, then run from the repository directory:

```sh
cargo install --path .
```

No configuration or environment variables are required. For OCR through the library's PDF parser, install Poppler's `pdftoppm` and Tesseract with Czech and English language data on `PATH`. The downloader only extracts PDF text layers.

## Use

Run from the directory containing your CSV files. `PO_ALL.csv` must have a header and company identifiers in its first column. Run batch processing before downloading:

```sh
batch_scraper
download_pdfs
```

To scan the fixed subject ID range separately:

```sh
justice_scraper
```

Each command accepts `--help` for its purpose and file defaults.

## License

[AGPL-3.0-or-later](LICENSE)
