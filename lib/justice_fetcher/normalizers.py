#
# Project: justice-scraper
# File:    normalizers.py
#
# Description:
# Text normalization and number and year parsing for the financial data.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import re
import unicodedata
from datetime import datetime, timezone
from typing import Any

# The registry prints the accounting period in square brackets and the filing
# dates after it:
#   "účetní závěrka [2025] , výroční zpráva [2025] ... 31.12.2025 8.7.2026"
# A bracketed year is therefore the period, and any year outside the brackets on
# such a row is a date the document was filed or published.
_BRACKETED_YEAR_RE = re.compile(r"\[\s*((?:19|20)\d{2})\s*\]")

_YEAR_RE = re.compile(r"\b(?:19|20)\d{2}\b")

# The registry itself starts in 1990.
_YEAR_LOWER_BOUND = 1990

# Currency written next to an amount. Matched after the text is folded to ASCII,
# so "Kč" arrives here as "kc".
_CURRENCY_RE = re.compile(r"(kc|czk)")


def normalize_text(value: str) -> str:
    return "".join(
        ch for ch in unicodedata.normalize("NFD", value) if unicodedata.category(ch) != "Mn"
    ).lower()


def extract_year(value: str) -> int | None:
    """The accounting period a document covers.

    A bracketed year wins outright. On a document row the brackets hold the
    period and the bare numbers after them are the balance sheet date and the
    filing dates, so taking the largest year on the row returned the year the
    document was filed. Every record came back one year late, and a statement
    filed in January came back for a period that had not ended.

    Without brackets the text is the statement itself, which names the period it
    covers and the one it compares against. The later of those is the period, so
    the largest year is right there. A year after this one is not a period any
    filed document can cover, so it is ignored.
    """
    bracketed = [int(y) for y in _BRACKETED_YEAR_RE.findall(value)]
    candidates = bracketed or [int(y) for y in _YEAR_RE.findall(value)]
    upper = datetime.now(tz=timezone.utc).year
    return max((y for y in candidates if _YEAR_LOWER_BOUND <= y <= upper), default=None)


def parse_number(raw: Any) -> float | None:
    if isinstance(raw, (int, float)):
        return float(raw)
    if not isinstance(raw, str):
        return None

    value = raw.strip()
    if not value:
        return None

    value = value.replace("\xa0", "").replace(" ", "")
    # The registry writes amounts in Kč. Fold the diacritic away before matching,
    # or the real symbol survives and the digit check below rejects the value.
    value = _CURRENCY_RE.sub("", normalize_text(value))
    is_negative_brackets = value.startswith("(") and value.endswith(")")
    if is_negative_brackets:
        value = value[1:-1]

    if re.search(r"[^0-9,.\-]", value):
        return None

    if "," in value and "." in value:
        if value.rfind(",") > value.rfind("."):
            value = value.replace(".", "").replace(",", ".")
        else:
            value = value.replace(",", "")
    elif "," in value:
        left, _, right = value.partition(",")
        value = f"{left}.{right}" if len(right) <= 2 else f"{left}{right}"
    elif "." in value:
        left, _, right = value.partition(".")
        if len(right) > 2:
            value = f"{left}{right}"

    try:
        parsed = float(value)
    except ValueError:
        return None

    return -parsed if is_negative_brackets else parsed
