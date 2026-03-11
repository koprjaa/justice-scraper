#
# Project: justice-scraper
# File:    html_parsers.py
#
# Description:
# Parses justice.cz HTML to extract subjektId, document candidates, and attachment URLs.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import re
from urllib.parse import urljoin

from bs4 import BeautifulSoup, Tag

from .config import JUSTICE_ORIGIN, MAX_DOCUMENT_CANDIDATES
from .models import DocumentCandidate
from .normalizers import extract_year, normalize_text


def extract_subjekt_id(html: str) -> str | None:
    soup = BeautifulSoup(html, "lxml")
    for anchor in soup.select("a[href*='vypis-sl-firma?subjektId=']"):
        href = anchor.get("href", "")
        match = re.search(r"[?&]subjektId=([^&]+)", href, flags=re.IGNORECASE)
        if match:
            return match.group(1)

    fallback = re.search(r"subjektId=([^\"&\s]+)", html, flags=re.IGNORECASE)
    return fallback.group(1) if fallback else None


def parse_document_candidates(html: str) -> list[DocumentCandidate]:
    soup = BeautifulSoup(html, "lxml")
    candidates: list[DocumentCandidate] = []
    seen: set[str] = set()

    for row in soup.select("tr"):
        row_text = " ".join(row.get_text(" ", strip=True).split())
        if "ucetni zaverka" not in normalize_text(row_text):
            continue

        link = row.select_one("a[href*='vypis-sl-detail?dokument=']") or row.select_one(
            "a[href*='dokument=']"
        )
        if not link:
            continue

        href = link.get("href", "")
        match = re.search(r"[?&]dokument=([^&]+)", href, flags=re.IGNORECASE)
        if not match:
            continue

        dokument_id = match.group(1)
        if dokument_id in seen:
            continue
        seen.add(dokument_id)

        candidates.append(
            DocumentCandidate(
                dokument_id=dokument_id,
                year=extract_year(row_text),
                detail_url=urljoin(f"{JUSTICE_ORIGIN}/ias/ui/", href),
            )
        )

    return sorted(candidates, key=_document_sort_key, reverse=True)[:MAX_DOCUMENT_CANDIDATES]


def extract_attachment_urls(
    detail_html: str,
    dokument_id: str | None = None,
) -> tuple[str | None, str | None, str | None]:
    links = _collect_attachment_links(detail_html)
    xml_url = _best_url(links, "xml", dokument_id)
    xhtml_url = _best_url(links, "xhtml", dokument_id)
    pdf_url = _best_url(links, "pdf", dokument_id)
    return xml_url, xhtml_url, pdf_url


def _collect_attachment_links(detail_html: str) -> list[tuple[str, str]]:
    # Section "Digitální podoba" (digital copy) on the detail page
    soup = BeautifulSoup(detail_html, "lxml")
    links: list[tuple[str, str]] = []
    seen: set[str] = set()

    header_cell = soup.find("th", string=lambda value: value and "Digitální podoba:" in value)
    if header_cell and isinstance(header_cell, Tag):
        row = header_cell.find_parent("tr")
        cursor = row.find_next_sibling("tr") if row else None
        for _ in range(12):
            if cursor is None or not isinstance(cursor, Tag):
                break
            if cursor.find("th"):
                break
            for anchor in cursor.select("a[href]"):
                href = anchor.get("href", "").strip()
                if not href or href in seen:
                    continue
                seen.add(href)
                text = " ".join(anchor.get_text(" ", strip=True).split())
                links.append((text, href))
            cursor = cursor.find_next_sibling("tr")

    if links:
        return links

    for anchor in soup.select("a[href]"):
        href = anchor.get("href", "").strip()
        if not href or href in seen:
            continue
        seen.add(href)
        text = " ".join(anchor.get_text(" ", strip=True).split())
        links.append((text, href))
    return links


def _best_url(
    links: list[tuple[str, str]],
    attachment_type: str,
    dokument_id: str | None,
) -> str | None:
    best_url: str | None = None
    best_score = 0

    for text, href in links:
        href_lower = href.lower()
        text_norm = normalize_text(text)
        score = _score_attachment(attachment_type, href_lower, text_norm, dokument_id)
        if score > best_score:
            best_score = score
            best_url = urljoin(f"{JUSTICE_ORIGIN}/ias/ui/", href)

    return best_url


def _score_attachment(
    attachment_type: str,
    href_lower: str,
    text_norm: str,
    dokument_id: str | None,
) -> int:
    score = 0

    if "ilinklistener-htmlcontainer-logo" in href_lower:
        return -100
    if any(token in href_lower or token in text_norm for token in ("gdpr", "ochrana", "informace")):
        return -60

    if "content/download" in href_lower or "/download/" in href_lower:
        score += 4
    if dokument_id and dokument_id in href_lower:
        score += 10

    if any(token in text_norm for token in ("zaverka", "ucetni", "rozvaha", "vysledovka", "vykaz")):
        score += 6

    if attachment_type == "xml":
        if ".xml" in href_lower or " xml" in text_norm or "xbrl" in text_norm:
            score += 20
        else:
            return 0
    elif attachment_type == "xhtml":
        if ".xhtml" in href_lower or "xhtml" in text_norm:
            score += 20
        else:
            return 0
    elif attachment_type == "pdf":
        if ".pdf" in href_lower or ".pdf" in text_norm or " pdf" in text_norm:
            score += 18
        else:
            return 0

    return score


def _document_sort_key(item: DocumentCandidate) -> tuple[int, int]:
    year = item.year or 0
    try:
        doc_num = int(item.dokument_id)
    except ValueError:
        doc_num = 0
    return year, doc_num
