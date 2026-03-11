# Project: justice-scraper
# File: xhtml_parser.py
# Description: Extracts financials from XHTML attachment content.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

from bs4 import BeautifulSoup

from .financial_text_parser import parse_financials_from_text


def parse_financial_xhtml(
    xhtml_text: str,
    fallback_year: int | None,
) -> dict[str, object]:
    soup = BeautifulSoup(xhtml_text, "lxml")
    normalized_text = soup.get_text("\n", strip=True)
    if not normalized_text.strip():
        return {
            "year": fallback_year,
            "revenue": None,
            "totalAssets": None,
            "netProfit": None,
            "confidence": "none",
            "sourceNotes": ["xhtml_empty_content"],
        }

    return parse_financials_from_text(
        normalized_text,
        fallback_year=fallback_year,
        source_notes=["parsed_from_xhtml"],
    )
