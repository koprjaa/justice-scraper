#
# Project: justice-scraper
# File:    test_financial_text_parser.py
#
# Description:
# Tests that the keyword based extraction does not claim confidence it has not earned.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Read against the live registry, this parser returned revenue of 1 for
Komerční banka and a profit larger than the revenue for ČEZ, and labelled both
high. The figures come from keyword proximity over arbitrary XHTML, so they will
sometimes be wrong. What they must not do is look reliable when they are not.
"""

from lib.justice_fetcher.financial_text_parser import (
    _implausible,
    _overall_confidence,
    parse_financials_from_text,
)

STATEMENT = """
Tržby z prodeje výrobků a služeb
15 000 000
Aktiva celkem
42 000 000
Výsledek hospodaření za účetní období
2 500 000
"""


def parse(text, row_year=2023):
    return parse_financials_from_text(text, row_year=row_year)


# --- overall confidence ------------------------------------------------------


def test_the_record_is_worth_its_weakest_figure():
    """Taking the best of the three let one good match carry two guesses."""
    assert _overall_confidence("high", "low", "medium") == "low"


def test_a_figure_that_was_never_found_does_not_drag_the_rest_down():
    assert _overall_confidence("high", "none", "high") == "high"


def test_nothing_found_at_all_is_no_confidence():
    assert _overall_confidence("none", "none", "none") == "none"


def test_a_single_found_figure_carries_its_own_level():
    assert _overall_confidence("none", "medium", "none") == "medium"


# --- the contradictions that mean a misread ----------------------------------


def test_a_profit_larger_than_the_revenue_is_flagged():
    assert "profit_exceeds_revenue" in _implausible(2022.0, None, 2618.0)


def test_a_loss_larger_than_the_revenue_is_flagged_too():
    assert "profit_exceeds_revenue" in _implausible(1000.0, None, -5000.0)


def test_a_profit_below_the_revenue_is_not_flagged():
    assert _implausible(15_000_000.0, 42_000_000.0, 2_500_000.0) == []


def test_revenue_negligible_against_the_assets_is_flagged():
    """Komerční banka came back with revenue of 1 against assets of 1.5 million."""
    assert "revenue_negligible_against_assets" in _implausible(1.0, 1_536_000.0, None)


def test_a_bare_year_read_as_revenue_is_flagged():
    assert "revenue_looks_like_a_year" in _implausible(2022.0, None, None)


def test_a_real_amount_near_the_year_range_is_not_flagged_as_a_year():
    assert "revenue_looks_like_a_year" not in _implausible(2022.5, None, None)


def test_missing_figures_raise_no_contradiction():
    assert _implausible(None, None, None) == []


# --- what the parser reports -------------------------------------------------


def test_a_consistent_statement_keeps_its_confidence():
    result = parse(STATEMENT)
    assert result["revenue"] == 15_000_000
    assert result["confidence"] != "low" or "profit_exceeds_revenue" not in result["sourceNotes"]


def test_a_contradictory_statement_is_downgraded_and_says_why():
    text = "Tržby\n100\nVýsledek hospodaření\n9 000 000\n"
    result = parse(text)
    assert result["confidence"] == "low"
    assert "profit_exceeds_revenue" in result["sourceNotes"]


def test_the_source_notes_that_were_passed_in_survive():
    result = parse_financials_from_text(
        "Tržby\n100\nVýsledek hospodaření\n9 000 000\n",
        row_year=2023,
        source_notes=["parsed_from_xhtml"],
    )
    assert "parsed_from_xhtml" in result["sourceNotes"]


def test_an_overlapping_keyword_does_not_score_twice():
    """"vysledek hospodareni" also contains "hospodareni". Counting both put a
    single line over the high threshold whatever number stood next to it."""
    result = parse("Výsledek hospodaření\n7\n")
    assert result["confidence"] != "high"
