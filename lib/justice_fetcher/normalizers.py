# Project: justice-scraper
# File: normalizers.py
# Description: Text normalization and number/year parsing for financial data.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

import re
import unicodedata
from datetime import datetime, timezone
from typing import Any

# Cached so extract_year() does not call datetime.now() every time
_YEAR_UPPER_BOUND = datetime.now(tz=timezone.utc).year + 1


def normalize_text(value: str) -> str:
    return "".join(
        ch for ch in unicodedata.normalize("NFD", value) if unicodedata.category(ch) != "Mn"
    ).lower()


def extract_year(value: str) -> int | None:
    years = [int(y) for y in re.findall(r"\b(?:19|20)\d{2}\b", value)]
    if not years:
        return None

    filtered = [year for year in years if 1990 <= year <= _YEAR_UPPER_BOUND]
    return max(filtered) if filtered else None


def parse_number(raw: Any) -> float | None:
    if isinstance(raw, (int, float)):
        return float(raw)
    if not isinstance(raw, str):
        return None

    value = raw.strip()
    if not value:
        return None

    value = value.replace("\xa0", "").replace(" ", "")
    value = re.sub(r"(Kc|CZK)", "", value, flags=re.IGNORECASE)
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
