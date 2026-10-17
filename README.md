# justice-scraper

Pulls financial statement links and figures from the Czech Justice Registry.

[![ci](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)

or.justice.cz has no public API, so this scrapes the HTML pages and the attached documents. It is a prototype. The parser matches Czech accounting labels rather than reading the statement schema, and it can return wrong numbers. Treat the figures as a lead, not as data.

## What it does

- `justice_scraper.py` walks a range of subject IDs and writes the first statement's document link to `document_links.csv`.
- `batch_scraper.py` reads company IDs (IČO) from `PO_ALL.csv` (header row, IČO in the first column) and writes document links to `financials_links.csv`.
- `download_pdfs.py` reads `financials_links.csv`, downloads the documents into `texts/` and extracts their text.
- `lib/justice_fetcher` returns year, revenue, total assets and net profit for one IČO. Records whose figures contradict each other are marked `confidence: low`.

## Install

```bash
pip install -r requirements.txt
```

Run every command from the repository root so that `lib` is importable.

## Use

```bash
python justice_scraper.py   # subject IDs 1 to 1000, edit START_ID and END_ID at the bottom of the file
python batch_scraper.py     # IČO list from PO_ALL.csv
python download_pdfs.py     # documents listed in financials_links.csv
```

From Python:

```python
import asyncio
from lib.justice_fetcher import fetch_justice_financials

print(asyncio.run(fetch_justice_financials("27074358")))
```

## License

[MIT](LICENSE)
