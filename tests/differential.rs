// Project: justice-scraper
// File: tests/differential.rs
// Description: Differential cases captured from the original Python implementation.
// Author: Jan Alexandr Kopřiva
// License: MIT
use justice_scraper::{html::*, parsers::*};
use serde_json::{json, Value};
#[test]
fn python_oracle() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/oracle.json")).unwrap();
    let mut failures = vec![];
    for (i, c) in cases.iter().enumerate() {
        let input = c["input"].as_str().unwrap();
        let actual = match c["kind"].as_str().unwrap() {
            "normalize" => json!(normalize_text(input)),
            "year" => json!(extract_year(input)),
            "number" => match parse_number(input) {
                Some(n) if n.is_infinite() => json!("infinity"),
                v => json!(v),
            },
            "text" => parse_financials_from_text(input, None, vec![]),
            "xml" => json!(parse_financial_xml(input, None)),
            "xhtml" => parse_financial_xhtml(input, None),
            "candidate" => json!(parse_document_candidates(input, ORIGIN)[0].dokument_id),
            _ => unreachable!(),
        };
        let expected = c.get("rustExpected").unwrap_or(&c["expected"]);
        if &actual != expected {
            failures.push(format!(
                "case {i} {} input={input:?}: Python={} Rust={actual}",
                c["kind"], c["expected"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
