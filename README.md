# justice-scraper

Reads the Czech Justice Registry (or.justice.cz) and extracts financial statement links and structured financial data. The registry has no public API, so the code scrapes HTML and downloads the attached documents.

![python](https://img.shields.io/badge/python-3.10+-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-prototype-lightgrey?style=flat-square)
[![ci](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/justice-scraper/actions/workflows/ci.yml)

## What it does

The repository holds two independent parts. They share an origin and a parsing style, but they do not call each other.

**Batch script (`justice_scraper.py`)** takes a numeric range of subject IDs. For each subject it fetches the page, finds the first *záverka* row, follows the detail page, and collects the digital copy link. XML is preferred over PDF. It writes one CSV with the columns `ico`, `vznik_listiny`, `link`, and `typ`.

**Library (`lib/justice_fetcher`)** is keyed by company identifier (IČO). It resolves the IČO to a subject ID, lists up to three *účetní závěrka* candidates, fetches each detail page, and parses XML first and XHTML second. From the first successful parse it returns year, revenue, total assets, and net profit.

## Install

```bash
pip install -r requirements.txt
```

Run every command from the project root so that `lib` is on the import path.

## Use

Batch script over a range of subject IDs:

```bash
python justice_scraper.py
```

It writes `document_links.csv` and logs to `scraper.log`. The default range is 1 to 1000. Change `START_ID` and `END_ID` at the bottom of `justice_scraper.py`.

Smoke test over subject IDs 1 to 10:

```bash
python test_scraper.py
```

Library, financial data by IČO:

```python
import asyncio
from lib.justice_fetcher import fetch_justice_financials

print(asyncio.run(fetch_justice_financials("27074358")))
```

The result is a dict with `ico`, `subjektId`, `financials`, and `lastUpdated`. Each financial record carries a confidence value and a source note.

## How it works

`service.py` runs three steps. It fetches the search page by IČO and parses the subject ID. It fetches the document list page and parses the candidates. For each candidate it fetches the detail page, reads the attachment URLs, and parses XML and then XHTML. If neither parses, the service returns a record with null values and source notes.

`JusticeHttpClient` in `http_client.py` performs all HTTP. It applies the rate limiter and the retry logic to navigation requests only. Attachment URLs point to a CDN and use a short timeout with no rate limit. Configuration lives in `config.py`. Data shapes live in `models.py`.

Three decisions are worth stating.

1. XML parses before XHTML. XML is structured and returns consistent fields. XHTML is a fallback. The first successful parse wins, so the code never merges two sources.
2. The library does not download or parse PDF attachments. This keeps the fetch fast.
3. The batch script turns TLS verification off, for environments with a corporate proxy or an old certificate chain. The library keeps TLS verification on.

## Limits

- The batch script iterates a numeric subject ID range. It does not accept a list of IČO values and it does not resolve an IČO to a subject ID. The mapping between IDs and companies is not documented here.
- A company that publishes PDF attachments only returns null financial values from the library.
- The parser depends on the current HTML and URL layout of or.justice.cz. A layout change breaks it.
- `test_scraper.py` prints its output and asserts nothing. It exercises the batch script against the live registry, so it is a smoke run rather than a test.
- The registry field names are Czech: IČO, záverka, vznik listiny, účetní závěrka.
- Attachment downloads have no rate limit. The repository does not handle CDN throttling.
- The parser does not know the statement schema. It walks every numeric node and scores the path by the Czech accounting labels along it. It does return wrong numbers. Read against the live registry it reported revenue of 1 for Komerční banka and a profit larger than the revenue for ČEZ. Treat the figures as a lead, not as data.
- A record whose figures contradict each other is marked `confidence: low` and carries a note naming the contradiction: `profit_exceeds_revenue`, `revenue_negligible_against_assets`, `revenue_looks_like_a_year`. These catch the shapes a misread produces, not every wrong number. Only `confidence: high` with no notes is worth anything, and even that is a keyword match rather than a schema read.
- Getting the figures right needs the structured statement rather than its rendered XHTML. That is a different piece of work and it is not in here.

## Development

```bash
uv run --extra dev ruff check .
uv run --extra dev pytest -q
```

The suite covers the number and year parsing and the XML scoring, with no
network. CI runs on Python 3.10, 3.11, and 3.12, across Linux and Windows.

## License

[MIT](LICENSE)
