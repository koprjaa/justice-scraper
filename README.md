# justice-scraper

[![Python](https://img.shields.io/badge/python-3.x-lightgrey?style=flat-square)](https://www.python.org/) [![aiohttp](https://img.shields.io/badge/aiohttp-async-lightgrey?style=flat-square)](https://github.com/aio-libs/aiohttp) [![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

## What it does

The repository contains two separate parts that consume the Czech Justice Registry (or.justice.cz). The root script `justice_scraper.py` takes a numeric range of subject IDs, fetches each subject’s page, locates the first "záverka" (closing report) row, follows the detail page, and collects the digital copy link (XML preferred, otherwise PDF). It writes one CSV with columns ico, vznik_listiny, link, typ. It does not resolve company identifier (IČO) to subject ID; it iterates subject IDs only. The library under `lib/justice_fetcher` is keyed by IČO: it resolves IČO to subject ID via the registry search (with a fallback URL), lists up to three "účetní závěrka" document candidates, fetches each detail page and attachment links, then downloads and parses XML, then XHTML, then PDF. From the first successful parse it extracts year, revenue, total assets, and net profit and returns a dict with ico, subjektId, financials (list of records with confidence and source notes), and lastUpdated. The two parts are not connected; they share the same origin and similar parsing patterns.

## Why it was built

The Czech Justice Registry does not expose an official API. The script and the library were built to obtain document links and structured financial data from the registry by scraping HTML and downloading attached documents.

## Architecture

**Root script (`justice_scraper.py`)**  
Single module: builds subject URLs from the ID range, uses aiohttp with a semaphore for concurrency, fetches HTML, parses with BeautifulSoup (lxml). For each subject it gets the main page, extracts IČO and the first záverka detail URL, fetches the detail page, parses the "Digitální podoba" section for XML or PDF link, and appends a row to a CSV. Logging goes to file and stdout. Output file and ID range are constants; TLS verification is disabled on the aiohttp connector.

**Library (`lib/justice_fetcher`)**  
`service.py` orchestrates a three-step flow: (1) fetch search page by IČO (primary URL, then fallback), parse HTML for subjektId via `html_parsers`; (2) fetch document list page by subjektId, parse document candidates (dokument_id, year, detail_url) with `html_parsers`; (3) for each candidate, fetch detail page, get attachment URLs via `html_parsers`, then fetch and parse in order XML (`xml_parser`), XHTML (`xhtml_parser`), PDF (`pdf_parser`). The first successful parse produces a `FinancialRecord`; records are collected into `JusticeFinancialsResult`. `JusticeHttpClient` in `http_client.py` performs all HTTP; it uses `AsyncRateLimiter` for navigation requests only and rotates User-Agent from config. Attachment requests use a longer timeout and no rate limit; navigation uses configurable rate limit and retries. Parsed financials come from `financial_text_parser` for PDF text; PDFs with little extracted text trigger OCR via pytesseract (see `pdf_parser`). Configuration lives in `config.py` (URLs, timeouts, rate limit, OCR threshold, User-Agents). Data shapes are in `models.py` (DocumentCandidate, FinancialRecord, JusticeFinancialsResult).

## Key decisions

**Parser order (XML → XHTML → PDF)**  
XML is structured and yields consistent fields; XHTML is semi-structured; PDF relies on text layer or OCR. The code tries each in order and uses the first successful parse so that the most reliable format wins without merging multiple sources.

**Rate limiting only on navigation**  
Attachment URLs point to a CDN. The client applies the rate limiter and retry logic only to navigation steps; attachment fetches use a longer timeout and no retries (per config) to avoid slowing large downloads and to treat CDN differently from the main site.

**SSL verification disabled in root script**  
A comment in `justice_scraper.py` states that TLS verification is turned off for environments where it fails (e.g. corporate proxy, old cert chain). The library does not disable SSL.

**OCR only when text layer is small**  
`config.py` defines `OCR_MIN_TEXT_CHARS = 120`. The PDF parser uses the text layer first; if the extracted text is shorter than that, it attempts Tesseract OCR. If Tesseract is missing or OCR fails, it continues with whatever text was extracted and records it in source notes.

**Rotating User-Agents in the library**  
`config.py` defines a list of User-Agent strings; `http_client.py` picks one at random per request. The comment in config says this is to reduce blocking.

**Two separate entry points**  
The root script is a batch job over a contiguous subject ID range and writes CSV. The library is keyed by IČO and returns a dict for programmatic use. They address different use cases and are not integrated.

## Trade-offs

**Root script: subject ID iteration**  
The script iterates a numeric subject ID range. It does not take a list of IČOs or resolve IČO to subject ID. Batch coverage is therefore by ID range, not by company list; gaps or mapping from IDs to companies are not documented in the repo.

**First successful parse wins**  
The library does not merge data from XML, XHTML, and PDF for the same document. It stops at the first format that parses successfully, so alternative or better data in another format is ignored.

**No rate limit on attachments**  
Faster attachment downloads were preferred. The repo does not mitigate possible CDN throttling or abuse detection on attachment URLs.

**SSL off in root script**  
Easier operation in locked-down or legacy environments was chosen over strict TLS in that script; using it on untrusted networks increases risk.

**OCR optional**  
PDF OCR depends on a system-installed Tesseract binary. The code does not require it; without it, PDFs with no or poor text layers yield best-effort or empty financial fields and the run continues.

## Limitations

Scraping depends on the current HTML and URL layout of or.justice.cz; structural or URL changes will break or require code updates. The root script does not document how subject IDs map to companies or whether the range contains gaps. The repository includes a smoke test (`test_scraper.py`) for the root scraper only; the financial fetcher has no tests. The registry and internal names (IČO, záverka, vznik_listiny, účetní závěrka) are in Czech. PDF parsing without a working Tesseract installation returns limited or no data for PDFs that lack a usable text layer.

## How to run

Install dependencies: `pip install -r requirements.txt`. Run from the project root so that `lib` is on the import path when using the fetcher.

**Root script (batch CSV):**  
`python justice_scraper.py`  
Writes `document_links.csv` and logs to `scraper.log`. The default subject ID range is 1–1000; change `START_ID` and `END_ID` at the bottom of `justice_scraper.py`.

**Smoke test:**  
`python test_scraper.py`  
Runs the root scraper for subject IDs 1–10 and prints the CSV; no automated assertions.

**Library (financials by IČO):**  
From project root: `python -c "import asyncio; from lib.justice_fetcher import fetch_justice_financials; print(asyncio.run(fetch_justice_financials('27074358')))"`  
Or import `fetch_justice_financials` in your code and run with an IČO string. Tesseract must be installed on the system if you need OCR fallback for PDFs with minimal text layer.
