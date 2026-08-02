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
    row_year: int | None,
    source_notes: list[str] | None = None,
) -> dict[str, object]:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    revenue, revenue_conf = _find_metric_from_lines(lines, ("trzby", "obrat", "vykony", "i."))
    assets, assets_conf = _find_metric_from_lines(lines, ("aktiva celkem", "aktiva", "b."))
    profit, profit_conf = _find_metric_from_lines(
        lines,
        ("vysledek hospodareni", "zisk", "ztrata", "hospodareni"),
    )

    confidence = _overall_confidence(revenue_conf, assets_conf, profit_conf)
    notes = list(source_notes or [])

    # A statement whose own figures contradict each other was misread, whatever
    # the keywords suggested. Komerční banka came back with revenue 1 against
    # assets of 1.5 million and was labelled high.
    failed = _implausible(revenue, assets, profit)
    if failed:
        notes.extend(failed)
        confidence = "low"

    return {
        # The registry's bracketed period beats any date inside the document.
        "year": row_year or extract_year(text),
        "revenue": revenue,
        "totalAssets": assets,
        "netProfit": profit,
        "confidence": confidence,
        "sourceNotes": notes,
    }


def _implausible(
    revenue: float | None, assets: float | None, profit: float | None
) -> list[str]:
    """Names every internal contradiction in a set of figures.

    These are not accounting rules, they are the shapes a misread produces. A
    real statement can be unusual, so a hit downgrades the confidence and says
    why rather than dropping the figures.
    """
    notes = []
    if revenue is not None and profit is not None and revenue > 0 and abs(profit) > revenue:
        notes.append("profit_exceeds_revenue")
    if revenue is not None and assets is not None and revenue > 0 and assets > revenue * 10_000:
        notes.append("revenue_negligible_against_assets")
    if revenue is not None and 1990 <= revenue <= 2100 and revenue == int(revenue):
        notes.append("revenue_looks_like_a_year")
    return notes


def _find_metric_from_lines(
    lines: list[str],
    keywords: tuple[str, ...],
) -> tuple[float | None, ConfidenceLevel]:
    best_value: float | None = None
    best_score = -1

    for index, line in enumerate(lines):
        normalized = normalize_text(line)
        # One line, one keyword match. Several of the keywords for a metric
        # overlap, so a line reading "vysledek hospodareni" used to match both
        # that phrase and "hospodareni" and score twice, which put it over the
        # high threshold on its own, whatever number stood next to it.
        if not any(keyword in normalized for keyword in keywords):
            continue
        score = _SCORE_KEYWORD_WEIGHT

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
    """The weakest figure that was actually found.

    Taking the best of the three let one well matched field carry a record whose
    other figures were guesses. A record is worth what its weakest number is
    worth. Fields that were not found at all say nothing either way, so they do
    not drag the rest down.
    """
    ordered: dict[ConfidenceLevel, int] = {"none": 0, "low": 1, "medium": 2, "high": 3}
    found = [p for p in parts if p != "none"]
    return min(found, key=lambda item: ordered[item], default="none")
