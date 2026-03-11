# justice-scraper

Scrapes the Czech Justice Registry (or.justice.cz) for company-related documents and, in a separate module, extracts structured financial data. There is no official API; both parts rely on HTML and document downloads.

## What it does

**Root script (`justice_scraper.py`)**  
Takes a numeric range of subject IDs (e.g. 1–1000). For each ID it fetches the subject’s page, finds the first “záverka” (closing report) row, follows the detail page, and collects the link to the digital copy (XML preferred, else PDF). Output is a single CSV: `ico`, `vznik_listiny`, `link`, `typ`. The script does not resolve company ID (IČO) to subject ID; it iterates subject IDs directly. Run from project root: `python justice_scraper.py`. Default range is 1–1000 (configurable via `START_ID` / `END_ID`).

**Library (`lib.justice_fetcher`)**  
Takes a company identifier (IČO). It resolves IČO to an internal subject ID via the registry search (with a fallback URL if the primary times out), lists “účetní závěrka” document candidates (up to three per company), then for each candidate fetches the detail page and attachment links. It downloads and parses in order: XML, then XHTML, then PDF. From each it extracts year, revenue, total assets, and net profit. Result is a dict: `ico`, `subjektId`, `financials` (list of records with confidence and source notes), `lastUpdated`. PDF parsing uses the text layer first; if the extracted text is below a character threshold (120), it falls back to OCR (requires a system-installed Tesseract binary). Attachment requests use a longer timeout and no rate limit; navigation requests are rate-limited and use rotating User-Agents.

## Why two parts

The root script is a batch job over a contiguous subject ID range and writes links to CSV. The library is keyed by IČO and returns structured financials for integration. They target different use cases and are not wired together; they share the same data source and similar parsing patterns.

## Dependencies

Listed in `requirements.txt`: aiohttp, beautifulsoup4, lxml, tqdm for the root scraper; the financial module also uses pdfplumber, pytesseract, pypdfium2, and Pillow. Install with `pip install -r requirements.txt`. Run from the project root so that `lib` is on the path when using the fetcher. Tuning for the root scraper (concurrency, timeout, retries, ID range) is in `justice_scraper.py`; for the fetcher (timeouts, rate limit, candidate count, OCR threshold) in `lib/justice_fetcher/config.py`.

## Usage

**Batch CSV (root):**
```bash
python justice_scraper.py
```
Output: `document_links.csv`, log: `scraper.log`.

**Smoke test (small ID range):**
```bash
python test_scraper.py
```
Runs the scraper for subject IDs 1–10 and prints the CSV contents. No automated assertions.

**Financial data by IČO (library):**
```python
import asyncio
from lib.justice_fetcher import fetch_justice_financials

result = asyncio.run(fetch_justice_financials("27074358"))
# result["financials"] = list of {year, revenue, totalAssets, netProfit, ...}
```

## Design choices

- **Parser order (XML → XHTML → PDF)**  
  XML is structured and gives consistent fields; XHTML is semi-structured; PDF is best-effort (text layer or OCR). The first successful parse wins.

- **Rate limiting only on navigation**  
  Attachment URLs point to a CDN; the code uses a longer timeout and no retries there, and does not apply the navigation rate limit to avoid slowing large file downloads.

- **SSL verification off in root scraper**  
  Comment in code cites environments where TLS verification fails (e.g. corporate proxy, old cert chain). Trade-off: simpler deployment in those environments, higher risk if used in untrusted networks.

- **OCR only when text layer is thin**  
  If the PDF text layer yields fewer than 120 characters, the module attempts Tesseract OCR. If Tesseract is missing or OCR fails, it continues with whatever text was extracted and records that in source notes.

## Limitations

- **HTML-bound**  
  All scraping depends on the current structure of or.justice.cz. Layout or URL changes will break or require updates.

- **Subject ID range**  
  The root script assumes a numeric subject ID range. The mapping from IDs to real companies and whether there are gaps is not documented in the repo.

- **No automated tests for the financial module**  
  The repo contains a smoke test for the root scraper only. The financial fetcher has no tests in the repository.

- **OCR optional**  
  PDF OCR requires Tesseract. Without it, PDFs with no or poor text layers return best-effort or empty data; the module does not crash.

- **Language**  
  The registry is in Czech; internal names (e.g. IČO, záverka, vznik_listiny) and parsed labels reflect that.
