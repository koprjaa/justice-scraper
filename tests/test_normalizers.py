#
# Project: justice-scraper
# File:    test_normalizers.py
#
# Description:
# Tests for the text, year, and number parsing behind the financial extraction.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Tests for the text, year and number parsing behind the financial extraction."""

from datetime import datetime, timezone

import pytest

from lib.justice_fetcher.normalizers import extract_year, normalize_text, parse_number

# --- normalize_text ---------------------------------------------------------


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        ("Tržby", "trzby"),
        ("AKTIVA CELKEM", "aktiva celkem"),
        ("Výsledek hospodaření", "vysledek hospodareni"),
        ("Kč", "kc"),
        ("", ""),
    ],
)
def test_diacritics_are_folded_and_the_text_lowercased(value, expected):
    assert normalize_text(value) == expected


def test_two_spellings_of_one_label_compare_equal():
    assert normalize_text("Tržby") == normalize_text("TRZBY")


# --- extract_year -----------------------------------------------------------


def test_the_year_is_read_out_of_free_text():
    assert extract_year("Účetní závěrka za rok 2023") == 2023


def test_the_latest_year_wins_when_several_appear():
    """A statement names the year it covers and the one it compares against."""
    assert extract_year("období 2022 a 2023") == 2023


# --- the accounting period versus the filing date ----------------------------

ROW = (
    "B 1581/SL315/MSPH účetní závěrka [2025] , výroční zpráva [2025] , "
    "zpráva o vztazích [2025] , zpráva auditora [2025] ve formátu XHTML "
    "31.12.2025 8.7.2026 13.7.2026"
)


def test_a_document_row_gives_the_period_not_the_filing_date():
    """The registry writes the period in brackets and the filing dates after it.

    Taking the largest year on the row returned 2026, the day the 2025 statement
    was filed, so every record was reported one year late.
    """
    assert extract_year(ROW) == 2025


def test_a_bracketed_year_beats_a_larger_bare_one():
    assert extract_year("závěrka [2021] uloženo 15.6.2024") == 2021


def test_the_latest_bracketed_year_wins_when_a_row_lists_several():
    assert extract_year("závěrka [2023] , výroční zpráva [2024] 30.6.2025") == 2024


def test_a_year_after_this_one_is_not_a_period_any_document_can_cover():
    next_year = datetime.now(tz=timezone.utc).year + 1
    assert extract_year(f"účetní závěrka za rok {next_year}") is None


def test_a_statement_that_mentions_a_future_year_falls_back_to_a_real_one():
    this_year = datetime.now(tz=timezone.utc).year
    text = f"srovnávací období {this_year - 1} a {this_year}, splatnost {this_year + 1}"
    assert extract_year(text) == this_year


def test_brackets_holding_something_other_than_a_year_are_ignored():
    assert extract_year("závěrka [SL315] za rok 2022") == 2022


@pytest.mark.parametrize("value", ["", "bez roku", "rok 1899", "rok 2100", "12345"])
def test_text_without_a_plausible_year_gives_none(value):
    assert extract_year(value) is None


def test_a_year_before_the_registry_existed_is_ignored():
    assert extract_year("1989") is None


# --- parse_number -----------------------------------------------------------


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("1000", 1000.0),
        ("1 000", 1000.0),
        ("1\xa0000", 1000.0),
        ("1 234,56", 1234.56),
        ("1.234,56", 1234.56),
        ("1,234.56", 1234.56),
        ("1.234", 1234.0),
        ("12,5", 12.5),
    ],
)
def test_the_number_formats_the_registry_uses(raw, expected):
    assert parse_number(raw) == pytest.approx(expected)


@pytest.mark.parametrize("raw", ["1 000 Kč", "1 000 KČ", "1000 Kc", "1000 CZK", "1000 czk"])
def test_an_amount_carrying_its_currency_still_parses(raw):
    """Kč is what the registry prints. Matching only "Kc" left every real
    amount with the symbol still attached, and the digit check then threw the
    whole value away as unparseable."""
    assert parse_number(raw) == 1000.0


@pytest.mark.parametrize("raw", ["(500)", "(500) Kč", "-500"])
def test_a_loss_comes_back_negative(raw):
    assert parse_number(raw) == -500.0


@pytest.mark.parametrize("raw", [42, 3.5, -7])
def test_a_value_that_is_already_a_number_passes_through(raw):
    assert parse_number(raw) == float(raw)


@pytest.mark.parametrize("raw", ["", "   ", None, [], {}, "abc", "12ab", "N/A", "-"])
def test_a_value_that_is_not_a_number_gives_none(raw):
    assert parse_number(raw) is None


def test_a_thousands_separator_is_not_read_as_a_decimal_point():
    """1.234 is one thousand two hundred, not one point two three four."""
    assert parse_number("1.234") == 1234.0
    assert parse_number("1.23") == 1.23
