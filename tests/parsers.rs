// Project: justice-scraper
// File: tests/parsers.rs
// Description: Equivalents of the original normalization and financial parser tests.
// Author: Jan Alexandr Kopřiva
// License: MIT
use chrono::{Datelike, Utc};
use justice_scraper::parsers::*;
#[test]
fn normalization() {
    for (s, want) in [
        ("Tržby", "trzby"),
        ("AKTIVA CELKEM", "aktiva celkem"),
        ("Výsledek hospodaření", "vysledek hospodareni"),
        ("Kč", "kc"),
        ("", ""),
    ] {
        assert_eq!(normalize_text(s), want);
    }
    assert_eq!(normalize_text("Tržby"), normalize_text("TRZBY"));
}
#[test]
fn years() {
    let now = Utc::now().year();
    for(s,want)in[("Účetní závěrka za rok 2023",Some(2023)),("období 2022 a 2023",Some(2023)),("B 1581/SL315/MSPH účetní závěrka [2025] , výroční zpráva [2025] , zpráva o vztazích [2025] , zpráva auditora [2025] ve formátu XHTML 31.12.2025 8.7.2026 13.7.2026",Some(2025)),("závěrka [2021] uloženo 15.6.2024",Some(2021)),("závěrka [2023] , výroční zpráva [2024] 30.6.2025",Some(2024)),("závěrka [SL315] za rok 2022",Some(2022)),("",None),("bez roku",None),("rok 1899",None),("rok 2100",None),("12345",None),("1989",None)]{assert_eq!(extract_year(s),want,"{s}");}
    assert_eq!(
        extract_year(&format!("účetní závěrka za rok {}", now + 1)),
        None
    );
    assert_eq!(
        extract_year(&format!(
            "srovnávací období {} a {now}, splatnost {}",
            now - 1,
            now + 1
        )),
        Some(now)
    );
    assert_eq!(extract_year(&format!("[{}] 2023", now + 1)), None);
}
#[test]
fn number_formats() {
    for (s, want) in [
        ("1000", 1000.),
        ("1 000", 1000.),
        ("1\u{a0}000", 1000.),
        ("1 234,56", 1234.56),
        ("1.234,56", 1234.56),
        ("1,234.56", 1234.56),
        ("1.234", 1234.),
        ("12,5", 12.5),
        ("1.23", 1.23),
        ("1 000 Kč", 1000.),
        ("1 000 KČ", 1000.),
        ("1000 Kc", 1000.),
        ("1000 CZK", 1000.),
        ("1000 czk", 1000.),
        ("(500)", -500.),
        ("(500) Kč", -500.),
        ("-500", -500.),
    ] {
        assert_eq!(parse_number(s), Some(want), "{s}");
    }
    for s in ["", "   ", "abc", "12ab", "N/A", "-", "[]", "{}"] {
        assert_eq!(parse_number(s), None);
    }
    for v in [
        serde_json::json!(42),
        serde_json::json!(3.5),
        serde_json::json!(-7),
    ] {
        assert_eq!(parse_number_value(&v), v.as_f64());
    }
    for v in [
        serde_json::Value::Null,
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        assert_eq!(parse_number_value(&v), None);
    }
}
#[test]
fn confidence() {
    assert_eq!(overall_confidence(&["high", "low", "medium"]), "low");
    assert_eq!(overall_confidence(&["high", "none", "high"]), "high");
    assert_eq!(overall_confidence(&["none", "none", "none"]), "none");
    assert_eq!(overall_confidence(&["none", "medium", "none"]), "medium");
}
#[test]
fn contradictions() {
    assert!(implausible(Some(2022.), None, Some(2618.)).contains(&"profit_exceeds_revenue".into()));
    assert!(implausible(Some(1000.), None, Some(-5000.)).contains(&"profit_exceeds_revenue".into()));
    assert!(implausible(Some(15000000.), Some(42000000.), Some(2500000.)).is_empty());
    assert!(implausible(Some(1.), Some(1536000.), None)
        .contains(&"revenue_negligible_against_assets".into()));
    assert!(implausible(Some(2022.), None, None).contains(&"revenue_looks_like_a_year".into()));
    assert!(!implausible(Some(2022.5), None, None).contains(&"revenue_looks_like_a_year".into()));
    assert!(implausible(None, None, None).is_empty());
}
#[test]
fn text_parser() {
    let r=parse_financials_from_text("Tržby z prodeje výrobků a služeb\n15 000 000\nAktiva celkem\n42 000 000\nVýsledek hospodaření za účetní období\n2 500 000",Some(2023),vec![]);
    assert_eq!(r["revenue"], 15000000.);
    assert!(
        r["confidence"] != "low"
            || !r["sourceNotes"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("profit_exceeds_revenue"))
    );
    let r = parse_financials_from_text(
        "Tržby\n100\nVýsledek hospodaření\n9 000 000\n",
        Some(2023),
        vec!["parsed_from_xhtml".into()],
    );
    assert_eq!(r["confidence"], "low");
    for n in ["profit_exceeds_revenue", "parsed_from_xhtml"] {
        assert!(r["sourceNotes"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(n)));
    }
    let r = parse_financials_from_text("Výsledek hospodaření\n7\n", Some(2023), vec![]);
    assert_ne!(r["confidence"], "high");
}
const XML: &str = include_str!("fixtures/statement.xml");
#[test]
fn xml_statement() {
    let r = parse_financial_xml(XML, None).unwrap();
    assert_eq!(r["revenue"], 15000000.);
    assert_eq!(r["totalAssets"], 42000000.);
    assert_eq!(r["netProfit"], 2500000.);
    assert_eq!(r["year"], 2023);
    assert_ne!(r["revenue"], 25.);
    assert_eq!(parse_financial_xml(XML, Some(2020)).unwrap()["year"], 2020);
    assert!(parse_financial_xml("<Vykaz><Trzby>", None).is_none());
    assert!(
        parse_financial_xml("<Vykaz><Trzby>100</Trzby></Vykaz>", None).unwrap()["year"].is_null()
    );
    let r = parse_financial_xml(
        "<Vykaz><Neco>123</Neco><Jine>456</Jine></Vykaz>",
        Some(2023),
    )
    .unwrap();
    for k in ["revenue", "totalAssets", "netProfit"] {
        assert!(r[k].is_null());
    }
    assert_eq!(
        parse_financial_xml("<Vykaz><Trzby>15 000 000 Kč</Trzby></Vykaz>", None).unwrap()
            ["revenue"],
        15000000.
    );
}
#[test]
fn xml_nodes() {
    let nodes = collect_numeric_nodes(XML).unwrap();
    assert!(nodes.contains(&("UcetniZaverka.Vykaz.Trzby".into(), 15000000.)));
    assert!(nodes.contains(&("UcetniZaverka.Vykaz.AktivaCelkem".into(), 42000000.)));
    assert!(
        collect_numeric_nodes("<Vykaz><Nazev>Acme s.r.o.</Nazev></Vykaz>")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        collect_numeric_nodes("<Vykaz xmlns='http://example.cz/ns'><Trzby>100</Trzby></Vykaz>")
            .unwrap(),
        vec![("Vykaz.Trzby".into(), 100.)]
    );
}
#[test]
fn xml_scoring_and_picking() {
    for (kind, path) in [
        (0, "Vykaz.Trzby"),
        (0, "Vykaz.Obrat"),
        (1, "Vykaz.AktivaCelkem"),
        (1, "Vykaz.Aktiva"),
        (2, "Vykaz.VysledekHospodareni"),
        (2, "Vykaz.Zisk"),
        (2, "Vykaz.Ztrata"),
    ] {
        assert!(score(path, kind) > 0);
    }
    for kind in 0..3 {
        assert_eq!(score("Vykaz.PocetZamestnancu", kind), 0);
    }
    assert!(score("AktivaCelkem", 1) > score("Aktiva", 1));
    assert_eq!(score("Tržby", 0), score("Trzby", 0));
    assert_eq!(
        pick_best(
            &[
                ("Vykaz.Aktiva".into(), 100.),
                ("Vykaz.AktivaCelkem".into(), 200.)
            ],
            1
        ),
        Some(200.)
    );
    assert_eq!(pick_best(&[("Vykaz.Neco".into(), 1.)], 0), None);
    assert_eq!(pick_best(&[], 0), None);
    assert_eq!(
        pick_best(&[("Vykaz.Trzby".into(), 1.), ("Jiny.Trzby".into(), 2.)], 0),
        Some(1.)
    );
}
#[test]
fn xhtml_and_invalid_pdf() {
    assert_eq!(
        parse_financial_xhtml("<html></html>", Some(2023))["sourceNotes"],
        serde_json::json!(["xhtml_empty_content"])
    );
    let r = parse_financial_xhtml("<p>Tržby</p><p>15 000 000</p>", Some(2020));
    assert_eq!(r["revenue"], 15000000.);
    assert_eq!(r["year"], 2020);
    assert!(extract_pdf_text(b"not a PDF", None).is_err());
}
#[test]
fn xml_entities_are_not_expanded() {
    assert!(parse_financial_xml(
        "<!DOCTYPE a [<!ENTITY amount '1000'>]><a><Trzby>&amount;</Trzby></a>",
        None
    )
    .is_none());
}
#[test]
fn pdf_text_preserves_page_content_without_renderer_padding() {
    let text = extract_pdf_text(include_bytes!("fixtures/statement.pdf"), None).unwrap();
    assert_eq!(text, "Revenue 15000000");
}
#[test]
fn deeply_nested_xml_does_not_recurse_on_the_stack() {
    let xml = format!(
        "{}<Trzby>100</Trzby>{}",
        "<a>".repeat(2000),
        "</a>".repeat(2000)
    );
    assert!(parse_financial_xml(&xml, None).is_none());
    let xml = format!(
        "{}<Trzby>100</Trzby>{}",
        "<a>".repeat(100),
        "</a>".repeat(100)
    );
    assert_eq!(parse_financial_xml(&xml, None).unwrap()["revenue"], 100.);
}
#[test]
fn invalid_limiter_intervals_do_not_panic() {
    for interval in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
        let _ = justice_scraper::http::RateLimiter::new(interval);
    }
}
#[cfg(unix)]
#[test]
fn external_command_timeout_kills_and_reaps_child() {
    let started = std::time::Instant::now();
    let error = command_output(
        std::process::Command::new("sleep").arg("10"),
        std::time::Duration::from_millis(40),
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}
#[test]
fn nonfinite_financial_json_is_null() {
    let xml = format!("<a><Trzby>{}</Trzby></a>", "9".repeat(400));
    assert!(parse_number(&"9".repeat(400)).unwrap().is_infinite());
    assert!(parse_financial_xml(&xml, None).unwrap()["revenue"].is_null());
}
#[test]
fn xml_depth_scanner_handles_quoted_angles_and_cdata() {
    let xml = "<?xml version='1.0'?><a attr='>'><!-- <a> --><Trzby><![CDATA[100]]></Trzby></a>";
    assert_eq!(parse_financial_xml(xml, None).unwrap()["revenue"], 100.);
    assert!(parse_financial_xml("<a attr='unterminated>", None).is_none());
    assert!(parse_financial_xml("<!-- unterminated", None).is_none());
}
