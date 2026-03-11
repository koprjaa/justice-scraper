# Project: justice-scraper
# File: xml_parser.py
# Description: Extracts revenue, assets, and profit from justice.cz financial XML.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

import xml.etree.ElementTree as ET
from typing import Callable

from .normalizers import extract_year, normalize_text, parse_number


def parse_financial_xml(xml_raw: str, fallback_year: int | None) -> dict[str, float | int | None] | None:
    try:
        root = ET.fromstring(xml_raw)
    except ET.ParseError:
        return None

    nodes = _collect_numeric_nodes(root)
    return {
        "year": extract_year(xml_raw) or fallback_year,
        "revenue": _pick_best(nodes, _score_revenue),
        "totalAssets": _pick_best(nodes, _score_assets),
        "netProfit": _pick_best(nodes, _score_profit),
    }


def _collect_numeric_nodes(node: ET.Element, path: str = "") -> list[tuple[str, float]]:
    nodes: list[tuple[str, float]] = []
    tag = node.tag.split("}")[-1] if "}" in node.tag else node.tag
    current_path = f"{path}.{tag}" if path else tag

    if node.text:
        value = parse_number(node.text)
        if value is not None:
            nodes.append((current_path, value))

    for child in node:
        nodes.extend(_collect_numeric_nodes(child, current_path))
    return nodes


def _score_revenue(path: str) -> int:
    norm = normalize_text(path)
    score = 0
    if "trzby" in norm:
        score += 12
    if "obrat" in norm:
        score += 10
    if "i." in norm:
        score += 3
    return score


def _score_assets(path: str) -> int:
    norm = normalize_text(path)
    score = 0
    if "aktivacelkem" in norm:
        score += 14
    if "aktiva" in norm:
        score += 9
    if "b." in norm:
        score += 3
    return score


def _score_profit(path: str) -> int:
    norm = normalize_text(path)
    score = 0
    if "vysledekhospodareni" in norm:
        score += 14
    if "hospodareni" in norm:
        score += 8
    if "zisk" in norm or "ztrata" in norm:
        score += 7
    return score


def _pick_best(
    nodes: list[tuple[str, float]],
    scorer: Callable[[str], int],
) -> float | None:
    best_score = -1
    best_value: float | None = None
    for path, value in nodes:
        score = scorer(path)
        if score > best_score:
            best_score = score
            best_value = value
    return best_value if best_score > 0 else None
