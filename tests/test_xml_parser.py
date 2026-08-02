#
# Project: justice-scraper
# File:    test_xml_parser.py
#
# Description:
# Tests for pulling revenue, assets, and profit out of a financial XML statement.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Tests for pulling revenue, assets and profit out of a financial XML statement.

The parser does not know the schema. It walks every numeric node, scores the
path by the Czech accounting labels along it, and takes the best scoring one.
"""

import pytest

from lib.justice_fetcher.xml_parser import (
    _collect_numeric_nodes,
    _pick_best,
    _score_assets,
    _score_profit,
    _score_revenue,
    parse_financial_xml,
)

STATEMENT = """<?xml version="1.0" encoding="utf-8"?>
<UcetniZaverka rok="2023">
  <Vykaz>
    <Trzby>15000000</Trzby>
    <AktivaCelkem>42000000</AktivaCelkem>
    <VysledekHospodareni>2500000</VysledekHospodareni>
    <PocetZamestnancu>25</PocetZamestnancu>
  </Vykaz>
</UcetniZaverka>"""


# --- whole statement --------------------------------------------------------


def test_the_three_figures_come_out_of_a_statement():
    result = parse_financial_xml(STATEMENT, fallback_year=None)
    assert result["revenue"] == 15000000
    assert result["totalAssets"] == 42000000
    assert result["netProfit"] == 2500000


def test_the_year_is_taken_from_the_document():
    assert parse_financial_xml(STATEMENT, fallback_year=2020)["year"] == 2023


def test_the_fallback_year_is_used_when_the_document_names_none():
    xml = "<Vykaz><Trzby>100</Trzby></Vykaz>"
    assert parse_financial_xml(xml, fallback_year=2019)["year"] == 2019


def test_malformed_xml_gives_none():
    assert parse_financial_xml("<Vykaz><Trzby>", fallback_year=None) is None


def test_a_statement_with_no_recognized_labels_reports_no_figures():
    xml = "<Vykaz><Neco>123</Neco><Jine>456</Jine></Vykaz>"
    result = parse_financial_xml(xml, fallback_year=2023)
    assert result["revenue"] is None
    assert result["totalAssets"] is None
    assert result["netProfit"] is None


def test_an_unrelated_number_is_not_reported_as_revenue():
    """Employee count sits in the same document and must not win."""
    assert parse_financial_xml(STATEMENT, fallback_year=None)["revenue"] != 25


def test_amounts_written_with_their_currency_are_read():
    xml = "<Vykaz><Trzby>15 000 000 Kč</Trzby></Vykaz>"
    assert parse_financial_xml(xml, fallback_year=None)["revenue"] == 15000000


# --- node collection --------------------------------------------------------


def test_every_numeric_node_is_collected_with_its_path():
    nodes = dict(_collect_numeric_nodes(_root(STATEMENT)))
    assert nodes["UcetniZaverka.Vykaz.Trzby"] == 15000000
    assert nodes["UcetniZaverka.Vykaz.AktivaCelkem"] == 42000000


def test_a_node_holding_text_is_skipped():
    nodes = _collect_numeric_nodes(_root("<Vykaz><Nazev>Acme s.r.o.</Nazev></Vykaz>"))
    assert nodes == []


def test_a_namespace_is_stripped_from_the_path():
    xml = '<Vykaz xmlns="http://example.cz/ns"><Trzby>100</Trzby></Vykaz>'
    paths = [path for path, _ in _collect_numeric_nodes(_root(xml))]
    assert paths == ["Vykaz.Trzby"]


def _root(xml):
    import xml.etree.ElementTree as ET

    return ET.fromstring(xml)


# --- scoring ----------------------------------------------------------------


@pytest.mark.parametrize(
    ("scorer", "path"),
    [
        (_score_revenue, "Vykaz.Trzby"),
        (_score_revenue, "Vykaz.Obrat"),
        (_score_assets, "Vykaz.AktivaCelkem"),
        (_score_assets, "Vykaz.Aktiva"),
        (_score_profit, "Vykaz.VysledekHospodareni"),
        (_score_profit, "Vykaz.Zisk"),
        (_score_profit, "Vykaz.Ztrata"),
    ],
)
def test_a_label_the_parser_knows_scores_above_zero(scorer, path):
    assert scorer(path) > 0


@pytest.mark.parametrize("scorer", [_score_revenue, _score_assets, _score_profit])
def test_an_unrelated_label_scores_zero(scorer):
    assert scorer("Vykaz.PocetZamestnancu") == 0


def test_the_more_specific_label_outscores_the_general_one():
    assert _score_assets("AktivaCelkem") > _score_assets("Aktiva")


def test_diacritics_do_not_change_the_score():
    assert _score_revenue("Tržby") == _score_revenue("Trzby")


# --- picking ----------------------------------------------------------------


def test_the_highest_scoring_node_wins():
    nodes = [("Vykaz.Aktiva", 100.0), ("Vykaz.AktivaCelkem", 200.0)]
    assert _pick_best(nodes, _score_assets) == 200.0


def test_nothing_is_returned_when_no_node_scores():
    assert _pick_best([("Vykaz.Neco", 1.0)], _score_revenue) is None


def test_an_empty_document_returns_nothing():
    assert _pick_best([], _score_revenue) is None


def test_a_tie_is_broken_by_document_order():
    """Equal scores keep the first node, so the result does not wobble."""
    nodes = [("Vykaz.Trzby", 1.0), ("Jiny.Trzby", 2.0)]
    assert _pick_best(nodes, _score_revenue) == 1.0
