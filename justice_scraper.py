#
# Project: justice-scraper
# File:    justice_scraper.py
#
# Description:
# Scrapes the Czech Justice Registry for subject IDs and writes záverka document links to CSV.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import asyncio
import csv
import logging
import re
import time
from pathlib import Path

import aiohttp
from bs4 import BeautifulSoup
from tqdm import tqdm

BASE_URL = "https://or.justice.cz/ias/ui/vypis-sl-firma?subjektId={subject_id}"
DETAIL_BASE = "https://or.justice.cz/ias/ui/"
ROOT_URL = "https://or.justice.cz"

MAX_CONCURRENT_REQUESTS = 20
REQUEST_TIMEOUT = 20
MAX_RETRIES = 3

OUTPUT_CSV = Path("document_links.csv")

HEADERS = {
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Safari/605.1.15",
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "cs,en;q=0.8",
    "Referer": "https://or.justice.cz/ias/ui/",
}

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)8s | %(message)s",
    handlers=[logging.FileHandler("scraper.log", "w", "utf-8"), logging.StreamHandler()],
)

# Registry row type "záverka" (closing report); regex allows spelling with/without diacritics.
ZAVERKA_RE = re.compile(r"z[áa]v[ěe]rka", re.IGNORECASE)


def normalize_url(href: str) -> str:
    if href.startswith("//"):
        return "https:" + href
    if href.startswith(("http://", "https://")):
        return href
    if href.startswith("./"):
        return DETAIL_BASE + href[2:]
    return ROOT_URL.rstrip("/") + "/" + href.lstrip("/")


async def fetch_html(session: aiohttp.ClientSession, url: str, retries: int = MAX_RETRIES) -> str | None:
    delay = 1.0
    for attempt in range(1, retries + 1):
        try:
            async with session.get(url, headers=HEADERS, allow_redirects=True) as response:
                if response.status == 200:
                    return await response.text()
                logging.warning("[%s] HTTP %s (attempt %s/%s)", url, response.status, attempt, retries)
        except (aiohttp.ClientError, asyncio.TimeoutError) as exc:
            logging.warning("[%s] %s (attempt %s/%s)", url, exc.__class__.__name__, attempt, retries)

        await asyncio.sleep(delay)
        delay = min(delay * 2, 8)

    return None


def parse_main_page(html: str) -> tuple[str | None, str | None, str | None]:
    soup = BeautifulSoup(html, "lxml")

    ico = None
    ico_label = soup.find("th", string=lambda value: value and "Identifikační číslo:" in value)
    if ico_label:
        row = ico_label.find_parent("tr")
        if row:
            ico_span = row.find("span", class_="nowrap")
            if ico_span:
                ico = ico_span.get_text(strip=True).replace(" ", "")

    detail_url = None
    vznik_listiny = None
    for symbol in soup.select("span.symbol"):
        if not ZAVERKA_RE.search(symbol.get_text(strip=True)):
            continue

        row = symbol.find_parent("tr")
        if not row:
            continue

        cells = row.find_all("td")
        if len(cells) < 3:
            continue

        link = cells[0].find("a", href=True)
        if link:
            detail_url = normalize_url(link["href"].strip())
            vznik_listiny = cells[2].get_text(strip=True)
            break

    return ico, detail_url, vznik_listiny


def parse_document_link(detail_html: str) -> tuple[str, str] | None:
    soup = BeautifulSoup(detail_html, "lxml")

    digital_section = soup.find("th", string=lambda value: value and "Digitální podoba:" in value)
    if not digital_section:
        return None

    section_row = digital_section.find_parent("tr")
    if not section_row:
        return None

    xml_url = None
    pdf_url = None
    row = section_row.find_next_sibling("tr")
    for _ in range(10):  # bound iteration on malformed HTML
        if row is None:
            break

        for link in row.find_all("a", href=True):
            href = link["href"].strip()
            text = link.get_text(strip=True).lower()
            href_lower = href.lower()
            full_url = normalize_url(href)

            if xml_url is None and (".xml" in text or ".xml" in href_lower):
                xml_url = full_url
            if pdf_url is None and (".pdf" in text or ".pdf" in href_lower):
                pdf_url = full_url

        row = row.find_next_sibling("tr")

    if xml_url:
        return xml_url, "XML"
    if pdf_url:
        return pdf_url, "PDF"
    return None


async def scrape_subject(
    session: aiohttp.ClientSession,
    semaphore: asyncio.Semaphore,
    subject_id: int,
) -> tuple[str, str, str, str] | None:
    async with semaphore:
        main_html = await fetch_html(session, BASE_URL.format(subject_id=f"{subject_id:06d}"))
        if not main_html:
            return None

        ico, detail_url, vznik_listiny = parse_main_page(main_html)
        if not ico or not detail_url:
            return None

        detail_html = await fetch_html(session, detail_url)
        if not detail_html:
            return None

        file_data = parse_document_link(detail_html)
        if not file_data:
            return None

        file_url, file_type = file_data
        return ico, vznik_listiny or "", file_url, file_type


async def scrape_range(start_id: int, end_id: int) -> int:
    semaphore = asyncio.Semaphore(MAX_CONCURRENT_REQUESTS)
    timeout = aiohttp.ClientTimeout(total=REQUEST_TIMEOUT)
    connector = aiohttp.TCPConnector(limit=MAX_CONCURRENT_REQUESTS, ttl_dns_cache=600)

    async with aiohttp.ClientSession(timeout=timeout, connector=connector) as session:
        tasks = [
            asyncio.create_task(scrape_subject(session, semaphore, subject_id))
            for subject_id in range(start_id, end_id + 1)
        ]

        written = 0
        with OUTPUT_CSV.open("w", newline="", encoding="utf-8") as output:
            writer = csv.writer(output)
            writer.writerow(["ico", "vznik_listiny", "link", "typ"])

            progress = tqdm(asyncio.as_completed(tasks), total=end_id - start_id + 1, desc="Scraping", unit="subjekt")
            for task in progress:
                try:
                    result = await task
                except Exception as exc:
                    logging.error("Task failed: %s", exc)
                    continue

                if result:
                    writer.writerow(result)
                    written += 1
                    progress.set_postfix(written=written)

        logging.info("Finished: saved %s rows to %s", written, OUTPUT_CSV)
        return written


if __name__ == "__main__":
    START_ID = 1
    END_ID = 1000

    start_time = time.perf_counter()
    asyncio.run(scrape_range(START_ID, END_ID))
    logging.info("Runtime: %.2fs", time.perf_counter() - start_time)
