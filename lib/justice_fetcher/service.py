#
# Project: justice-scraper
# File:    service.py
#
# Description:
# Orchestrates the three-step fetch (subjektId, document list, attachments) and parses financials from XML/XHTML/PDF.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import asyncio
import re
from datetime import datetime, timezone
from typing import cast

import aiohttp

from .config import (
    RATE_LIMIT_SECONDS,
    REQUEST_TIMEOUT_SECONDS,
    STEP1_URL_FALLBACK,
    STEP1_URL_PRIMARY,
    STEP2_URL,
    STEP3_URL,
)
from .html_parsers import (
    extract_attachment_urls,
    extract_subjekt_id,
    parse_document_candidates,
)
from .http_client import JusticeHttpClient
from .logging_utils import log_status
from .models import ConfidenceLevel, DocumentCandidate, FinancialRecord, JusticeFinancialsResult, empty_result
from .pdf_parser import parse_financial_pdf
from .rate_limiter import AsyncRateLimiter
from .xhtml_parser import parse_financial_xhtml
from .xml_parser import parse_financial_xml


async def fetch_justice_financials(ico: str) -> dict[str, object]:
    sanitized_ico = re.sub(r"\D", "", ico or "")[:8]
    if not sanitized_ico:
        log_status("justice-scraper ico=%s status=invalid_ico", ico)
        return empty_result(ico or "").to_dict()

    log_status("justice-scraper ico=%s status=start", sanitized_ico)

    timeout = aiohttp.ClientTimeout(total=REQUEST_TIMEOUT_SECONDS)
    connector = aiohttp.TCPConnector(
        limit=0,
        ttl_dns_cache=300,
    )
    headers = {
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "cs,en;q=0.8",
        "Connection": "keep-alive",
    }
    limiter = AsyncRateLimiter(min_interval_seconds=RATE_LIMIT_SECONDS)

    async with aiohttp.ClientSession(
        timeout=timeout,
        headers=headers,
        connector=connector,
    ) as session:
        client = JusticeHttpClient(session, limiter)
        return await _fetch_financials_with_client(client, sanitized_ico)


async def _fetch_financials_with_client(client: JusticeHttpClient, ico: str) -> dict[str, object]:
    step1_html = await client.fetch_text(
        STEP1_URL_PRIMARY.format(ico=ico), ico, "step1_primary"
    )
    if step1_html is None:
        step1_html = await client.fetch_text(
            STEP1_URL_FALLBACK.format(ico=ico), ico, "step1_fallback"
        )
    if step1_html is None:
        log_status("justice-scraper ico=%s status=step1_failed", ico)
        return empty_result(ico).to_dict()

    subjekt_id = extract_subjekt_id(step1_html)
    if not subjekt_id:
        log_status("justice-scraper ico=%s status=subjekt_not_found", ico)
        return empty_result(ico).to_dict()

    list_html = await client.fetch_text(
        STEP2_URL.format(subjekt_id=subjekt_id), ico, "step2_documents"
    )
    if list_html is None:
        log_status("justice-scraper ico=%s subjektId=%s status=documents_failed", ico, subjekt_id)
        return empty_result(ico, subjekt_id).to_dict()

    candidates = parse_document_candidates(list_html)
    if not candidates:
        log_status("justice-scraper ico=%s subjektId=%s status=no_financial_documents", ico, subjekt_id)
        return empty_result(ico, subjekt_id).to_dict()

    current_year = datetime.now(tz=timezone.utc).year
    records = await asyncio.gather(
        *[_process_candidate(client, ico, candidate, current_year) for candidate in candidates],
        return_exceptions=False,
    )

    financials = sorted(records, key=lambda r: r.year, reverse=True)
    result = JusticeFinancialsResult(
        ico=ico,
        subjekt_id=subjekt_id,
        financials=financials,
        last_updated=datetime.now(tz=timezone.utc),
    )
    log_status(
        "justice-scraper ico=%s subjektId=%s status=done records=%s",
        ico,
        subjekt_id,
        len(financials),
    )
    return result.to_dict()


async def _process_candidate(
    client: JusticeHttpClient,
    ico: str,
    candidate: DocumentCandidate,
    current_year: int,
) -> FinancialRecord:
    detail_url = candidate.detail_url or STEP3_URL.format(dokument_id=candidate.dokument_id)
    fallback_year = candidate.year or current_year

    detail_html = await client.fetch_text(detail_url, ico, f"step3_detail_{candidate.dokument_id}")
    if detail_html is None:
        return _fallback_record(fallback_year, detail_url, ["document_detail_unavailable"])

    xml_url, xhtml_url, pdf_url = extract_attachment_urls(detail_html, candidate.dokument_id)

    if xml_url:
        xml_text = await client.fetch_text(xml_url, ico, f"step3_xml_{candidate.dokument_id}", referer=detail_url)
        if xml_text is not None:
            parsed = parse_financial_xml(xml_text, candidate.year)
            if parsed is not None:
                return FinancialRecord(
                    year=int(parsed["year"] or fallback_year),
                    revenue=_as_float(parsed["revenue"]),
                    total_assets=_as_float(parsed["totalAssets"]),
                    net_profit=_as_float(parsed["netProfit"]),
                    source_type="xml",
                    document_url=xml_url,
                    parser_used="xml",
                    confidence="high",
                    source_notes=["parsed_from_xml"],
                )

    if xhtml_url:
        xhtml_text = await client.fetch_text(xhtml_url, ico, f"step3_xhtml_{candidate.dokument_id}", referer=detail_url)
        if xhtml_text is not None:
            parsed_xhtml = parse_financial_xhtml(xhtml_text, candidate.year)
            return FinancialRecord(
                year=int(parsed_xhtml["year"] or fallback_year),
                revenue=_as_float(parsed_xhtml["revenue"]),
                total_assets=_as_float(parsed_xhtml["totalAssets"]),
                net_profit=_as_float(parsed_xhtml["netProfit"]),
                source_type="xhtml",
                document_url=xhtml_url,
                parser_used="xhtml",
                confidence=_coerce_confidence(parsed_xhtml.get("confidence")),
                source_notes=_coerce_notes(parsed_xhtml.get("sourceNotes")),
            )

    if pdf_url:
        pdf_bytes = await client.fetch_bytes(pdf_url, ico, f"step3_pdf_{candidate.dokument_id}", referer=detail_url)
        if pdf_bytes is not None:
            parsed_pdf = parse_financial_pdf(pdf_bytes, candidate.year)
            return FinancialRecord(
                year=int(parsed_pdf["year"] or fallback_year),
                revenue=_as_float(parsed_pdf["revenue"]),
                total_assets=_as_float(parsed_pdf["totalAssets"]),
                net_profit=_as_float(parsed_pdf["netProfit"]),
                source_type="pdf",
                document_url=pdf_url,
                parser_used="pdf",
                confidence=_coerce_confidence(parsed_pdf.get("confidence")),
                source_notes=_coerce_notes(parsed_pdf.get("sourceNotes")),
            )
        return _fallback_record(fallback_year, pdf_url, ["pdf_download_failed"])

    return _fallback_record(fallback_year, detail_url, ["no_attachment_found"])


def _fallback_record(year: int, document_url: str, source_notes: list[str]) -> FinancialRecord:
    return FinancialRecord(
        year=year,
        revenue=None,
        total_assets=None,
        net_profit=None,
        source_type="pdf",
        document_url=document_url,
        parser_used="fallback",
        confidence="none",
        source_notes=source_notes,
    )


def _as_float(value: object) -> float | None:
    if isinstance(value, (int, float)):
        return float(value)
    return None


def _coerce_confidence(value: object) -> ConfidenceLevel:
    if value in {"high", "medium", "low", "none"}:
        return cast(ConfidenceLevel, value)
    return "none"


def _coerce_notes(value: object) -> list[str]:
    if isinstance(value, list):
        return [str(item) for item in value]
    return []
