#
# Project: justice-scraper
# File:    models.py
#
# Description:
# Data classes for document candidates, financial records, and API result shape.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Literal

SourceType = Literal["xml", "xhtml", "pdf"]
ConfidenceLevel = Literal["high", "medium", "low", "none"]


@dataclass(frozen=True)
class DocumentCandidate:
    dokument_id: str
    year: int | None
    detail_url: str


@dataclass(frozen=True)
class FinancialRecord:
    year: int
    revenue: float | None
    total_assets: float | None
    net_profit: float | None
    source_type: SourceType
    document_url: str
    dokument_id: str | None = None
    parser_used: str = "none"
    confidence: ConfidenceLevel = "none"
    source_notes: list[str] | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "year": self.year,
            "revenue": self.revenue,
            "totalAssets": self.total_assets,
            "netProfit": self.net_profit,
            "sourceType": self.source_type,
            "documentUrl": self.document_url,
            "dokumentId": self.dokument_id,
            "parserUsed": self.parser_used,
            "confidence": self.confidence,
            "sourceNotes": self.source_notes or [],
        }


@dataclass(frozen=True)
class JusticeFinancialsResult:
    ico: str
    subjekt_id: str
    financials: list[FinancialRecord]
    last_updated: datetime

    def to_dict(self) -> dict[str, Any]:
        return {
            "ico": self.ico,
            "subjektId": self.subjekt_id,
            "financials": [item.to_dict() for item in self.financials],
            "lastUpdated": self.last_updated.isoformat(),  # ISO-8601 for JSON
        }


def empty_result(ico: str, subjekt_id: str = "") -> JusticeFinancialsResult:
    return JusticeFinancialsResult(
        ico=ico,
        subjekt_id=subjekt_id,
        financials=[],
        last_updated=datetime.now(tz=timezone.utc),
    )
