#
# Project: justice-scraper
# File:    xhtml_parser.py
#
# Description:
# Extracts the financial figures from an XHTML attachment.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

from bs4 import BeautifulSoup

from .financial_text_parser import parse_financials_from_text


def parse_financial_xhtml(
    xhtml_text: str,
    row_year: int | None,
) -> dict[str, object]:
    """Figures out of an XHTML statement.

    row_year is the accounting period the registry printed in brackets on the
    document row. It is authoritative, so it wins over any year found inside the
    document, which also carries filing and signature dates.
    """
    soup = BeautifulSoup(xhtml_text, "lxml")
    normalized_text = soup.get_text("\n", strip=True)
    if not normalized_text.strip():
        return {
            "year": row_year,
            "revenue": None,
            "totalAssets": None,
            "netProfit": None,
            "confidence": "none",
            "sourceNotes": ["xhtml_empty_content"],
        }

    return parse_financials_from_text(
        normalized_text,
        row_year=row_year,
        source_notes=["parsed_from_xhtml"],
    )
