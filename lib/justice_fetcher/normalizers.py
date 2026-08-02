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

# Cached to avoid repeated datetime.now() in extract_year()
_YEAR_UPPER_BOUND = datetime.now(tz=timezone.utc).year + 1

# Currency written next to an amount. Matched after the text is folded to ASCII,
# so "Kč" arrives here as "kc".
_CURRENCY_RE = re.compile(r"(kc|czk)")


def normalize_text(value: str) -> str:
    return "".join(
        ch for ch in unicodedata.normalize("NFD", value) if unicodedata.category(ch) != "Mn"
    ).lower()


def extract_year(value: str) -> int | None:
    years = [int(y) for y in re.findall(r"\b(?:19|20)\d{2}\b", value)]
    return max((y for y in years if 1990 <= y <= _YEAR_UPPER_BOUND), default=None)


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
