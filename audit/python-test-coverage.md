# Original test coverage

All 91 parametrized cases passed under Python 3.11 before removal. Related cases are grouped in Rust tests; each original assertion has an equivalent.

| Original Python test | Rust equivalent |
| --- | --- |
| `test_financial_text_parser.py::test_the_record_is_worth_its_weakest_figure` | `tests/parsers.rs::confidence` |
| `test_financial_text_parser.py::test_a_figure_that_was_never_found_does_not_drag_the_rest_down` | `tests/parsers.rs::confidence` |
| `test_financial_text_parser.py::test_nothing_found_at_all_is_no_confidence` | `tests/parsers.rs::confidence` |
| `test_financial_text_parser.py::test_a_single_found_figure_carries_its_own_level` | `tests/parsers.rs::confidence` |
| `test_financial_text_parser.py::test_a_profit_larger_than_the_revenue_is_flagged` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_a_loss_larger_than_the_revenue_is_flagged_too` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_a_profit_below_the_revenue_is_not_flagged` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_revenue_negligible_against_the_assets_is_flagged` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_a_bare_year_read_as_revenue_is_flagged` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_a_real_amount_near_the_year_range_is_not_flagged_as_a_year` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_missing_figures_raise_no_contradiction` | `tests/parsers.rs::contradictions` |
| `test_financial_text_parser.py::test_a_consistent_statement_keeps_its_confidence` | `tests/parsers.rs::text_parser` |
| `test_financial_text_parser.py::test_a_contradictory_statement_is_downgraded_and_says_why` | `tests/parsers.rs::text_parser` |
| `test_financial_text_parser.py::test_the_source_notes_that_were_passed_in_survive` | `tests/parsers.rs::text_parser` |
| `test_financial_text_parser.py::test_an_overlapping_keyword_does_not_score_twice` | `tests/parsers.rs::text_parser` |
| `test_normalizers.py::test_diacritics_are_folded_and_the_text_lowercased` | `tests/parsers.rs::normalization` |
| `test_normalizers.py::test_two_spellings_of_one_label_compare_equal` | `tests/parsers.rs::normalization` |
| `test_normalizers.py::test_the_year_is_read_out_of_free_text` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_the_latest_year_wins_when_several_appear` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_a_document_row_gives_the_period_not_the_filing_date` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_a_bracketed_year_beats_a_larger_bare_one` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_the_latest_bracketed_year_wins_when_a_row_lists_several` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_a_year_after_this_one_is_not_a_period_any_document_can_cover` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_a_statement_that_mentions_a_future_year_falls_back_to_a_real_one` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_brackets_holding_something_other_than_a_year_are_ignored` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_text_without_a_plausible_year_gives_none` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_a_year_before_the_registry_existed_is_ignored` | `tests/parsers.rs::years` |
| `test_normalizers.py::test_the_number_formats_the_registry_uses` | `tests/parsers.rs::number_formats` |
| `test_normalizers.py::test_an_amount_carrying_its_currency_still_parses` | `tests/parsers.rs::number_formats` |
| `test_normalizers.py::test_a_loss_comes_back_negative` | `tests/parsers.rs::number_formats` |
| `test_normalizers.py::test_a_value_that_is_already_a_number_passes_through` | `tests/parsers.rs::number_formats` |
| `test_normalizers.py::test_a_value_that_is_not_a_number_gives_none` | `tests/parsers.rs::number_formats` |
| `test_normalizers.py::test_a_thousands_separator_is_not_read_as_a_decimal_point` | `tests/parsers.rs::number_formats` |
| `test_xml_parser.py::test_the_three_figures_come_out_of_a_statement` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_the_period_from_the_registry_row_wins_over_the_document` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_the_document_year_is_used_when_the_row_names_no_period` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_no_year_anywhere_gives_none` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_malformed_xml_gives_none` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_a_statement_with_no_recognized_labels_reports_no_figures` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_an_unrelated_number_is_not_reported_as_revenue` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_amounts_written_with_their_currency_are_read` | `tests/parsers.rs::xml_statement` |
| `test_xml_parser.py::test_every_numeric_node_is_collected_with_its_path` | `tests/parsers.rs::xml_nodes` |
| `test_xml_parser.py::test_a_node_holding_text_is_skipped` | `tests/parsers.rs::xml_nodes` |
| `test_xml_parser.py::test_a_namespace_is_stripped_from_the_path` | `tests/parsers.rs::xml_nodes` |
| `test_xml_parser.py::test_a_label_the_parser_knows_scores_above_zero` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_an_unrelated_label_scores_zero` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_the_more_specific_label_outscores_the_general_one` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_diacritics_do_not_change_the_score` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_the_highest_scoring_node_wins` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_nothing_is_returned_when_no_node_scores` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_an_empty_document_returns_nothing` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_xml_parser.py::test_a_tie_is_broken_by_document_order` | `tests/parsers.rs::xml_scoring_and_picking` |
| `test_scraper.py::test_scraper` | `tests/network.rs::range_smoke_and_csv_bytes` (loopback HTTP, 10 IDs, exact CSV bytes) |
