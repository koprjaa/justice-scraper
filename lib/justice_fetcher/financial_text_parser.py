#
# Project: justice-scraper
# File:    financial_text_parser.py
#
# Description:
# Keyword based extraction of revenue, assets, and profit from plain text.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import re

from .models import ConfidenceLevel
from .normalizers import extract_year, normalize_text, parse_number

_SCORE_KEYWORD_WEIGHT = 6
_CONFIDENCE_HIGH_THRESHOLD = 12
_CONFIDENCE_MEDIUM_THRESHOLD = 8


def parse_financials_from_text(
    text: str,
    fallback_year: int | None,
    source_notes: list[str] | None = None,
) -> dict[str, object]:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    revenue, revenue_conf = _find_metric_from_lines(lines, ("trzby", "obrat", "vykony", "i."))
    assets, assets_conf = _find_metric_from_lines(lines, ("aktiva celkem", "aktiva", "b."))
    profit, profit_conf = _find_metric_from_lines(
        lines,
        ("vysledek hospodareni", "zisk", "ztrata", "hospodareni"),
    )

    return {
        "year": extract_year(text) or fallback_year,
        "revenue": revenue,
        "totalAssets": assets,
        "netProfit": profit,
        "confidence": _overall_confidence(revenue_conf, assets_conf, profit_conf),
        "sourceNotes": source_notes or [],
    }


def _find_metric_from_lines(
    lines: list[str],
    keywords: tuple[str, ...],
) -> tuple[float | None, ConfidenceLevel]:
    best_value: float | None = None
    best_score = -1

    for index, line in enumerate(lines):
        normalized = normalize_text(line)
        score = sum(_SCORE_KEYWORD_WEIGHT for keyword in keywords if keyword in normalized)
        if score == 0:
            continue

        numbers = _extract_numbers(line)
        if not numbers and index + 1 < len(lines):
            numbers = _extract_numbers(lines[index + 1])
            if numbers:
                score += 2

        for number in numbers:
            candidate_score = score + _score_number_candidate(number)
            if candidate_score > best_score:
                best_score = candidate_score
                best_value = number

    if best_value is None:
        return None, "none"
    if best_score >= _CONFIDENCE_HIGH_THRESHOLD:
        return best_value, "high"
    if best_score >= _CONFIDENCE_MEDIUM_THRESHOLD:
        return best_value, "medium"
    return best_value, "low"


def _extract_numbers(text: str) -> list[float]:
    tokens = re.findall(r"\(?-?\d[\d\s.,]{0,20}\)?", text)
    return [x for t in tokens if (x := parse_number(t)) is not None]


def _score_number_candidate(value: float) -> int:
    magnitude = abs(value)
    if magnitude >= 1_000_000:
        return 5
    if magnitude >= 100_000:
        return 3
    if magnitude >= 10_000:
        return 2
    return 1


def _overall_confidence(*parts: ConfidenceLevel) -> ConfidenceLevel:
    ordered: dict[ConfidenceLevel, int] = {"none": 0, "low": 1, "medium": 2, "high": 3}
    return max(parts, key=lambda item: ordered[item], default="none")
