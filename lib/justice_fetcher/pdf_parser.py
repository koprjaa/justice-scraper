#
# Project: justice-scraper
# File:    pdf_parser.py
#
# Description:
# Reads text from a PDF attachment, by text layer or by OCR, and parses the financials out of it.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

from __future__ import annotations

import io

from .config import OCR_MIN_TEXT_CHARS
from .financial_text_parser import parse_financials_from_text


def parse_financial_pdf(
    pdf_bytes: bytes,
    row_year: int | None,
) -> dict[str, object]:
    source_notes: list[str] = []
    text_content = _extract_pdf_text(pdf_bytes, source_notes)

    if len(text_content.strip()) < OCR_MIN_TEXT_CHARS:
        ocr_text = _extract_pdf_text_with_ocr(pdf_bytes, source_notes)
        if ocr_text:
            text_content = f"{text_content}\n{ocr_text}".strip()

    if not text_content.strip():
        return {
            "year": row_year,
            "revenue": None,
            "totalAssets": None,
            "netProfit": None,
            "confidence": "none",
            "sourceNotes": source_notes or ["no_text_extracted_from_pdf"],
        }

    return parse_financials_from_text(
        text_content,
        row_year=row_year,
        source_notes=source_notes or ["parsed_from_pdf_text"],
    )


def _extract_pdf_text(pdf_bytes: bytes, source_notes: list[str]) -> str:
    try:
        import pdfplumber
    except Exception:
        source_notes.append("pdfplumber_unavailable")
        return ""

    chunks: list[str] = []
    try:
        with pdfplumber.open(io.BytesIO(pdf_bytes)) as pdf:
            for page in pdf.pages[:8]:
                page_text = page.extract_text() or ""
                if page_text.strip():
                    chunks.append(page_text)
        source_notes.append("pdf_text_layer_used")
        return "\n".join(chunks)
    except Exception:
        source_notes.append("pdf_text_extraction_failed")
        return ""


def _extract_pdf_text_with_ocr(pdf_bytes: bytes, source_notes: list[str]) -> str:
    try:
        import pypdfium2 as pdfium
        import pytesseract
    except Exception:
        source_notes.append("ocr_dependencies_unavailable")
        return ""

    try:
        pytesseract.get_tesseract_version()
    except Exception:
        source_notes.append("tesseract_binary_unavailable")
        return ""

    chunks: list[str] = []
    try:
        pdf = pdfium.PdfDocument(pdf_bytes)
        for page_index in range(min(len(pdf), 5)):
            text = pytesseract.image_to_string(
                pdf[page_index].render(scale=2.0).to_pil(), lang="ces+eng"
            )
            if text.strip():
                chunks.append(text)
        if chunks:
            source_notes.append("ocr_used")
        return "\n".join(chunks)
    except Exception:
        source_notes.append("ocr_failed")
        return ""
