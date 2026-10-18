// Project: justice-scraper
// File: src/parsers.rs
// Description: Locale normalization and financial document extraction.
// Author: Jan Alexandr Kopřiva
// License: MIT
use chrono::{Datelike, Utc};
use regex::Regex;
use serde_json::{json, Value};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn normalize_text(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}
pub fn extract_year(s: &str) -> Option<i32> {
    let bracket = Regex::new(r"\[\s*((?:19|20)\d{2})\s*\]").unwrap();
    let plain = Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();
    let mut years: Vec<i32> = bracket
        .captures_iter(s)
        .filter_map(|c| c[1].parse().ok())
        .collect();
    if years.is_empty() {
        years = plain
            .captures_iter(s)
            .filter_map(|c| c[1].parse().ok())
            .collect();
    }
    years
        .into_iter()
        .filter(|y| (1990..=Utc::now().year()).contains(y))
        .max()
}
pub fn parse_number(s: &str) -> Option<f64> {
    let mut s = normalize_text(s.trim())
        .replace([' ', '\u{a0}'], "")
        .replace("kc", "")
        .replace("czk", "");
    let negative = s.starts_with('(') && s.ends_with(')');
    if negative {
        s = s[1..s.len() - 1].to_owned();
    }
    if s.chars().any(|c| !c.is_ascii_digit() && !",.-".contains(c)) {
        return None;
    }
    match (s.rfind(','), s.rfind('.')) {
        (Some(c), Some(d)) if c > d => s = s.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => s = s.replace(',', ""),
        (Some(_), None) => {
            let (l, r) = s.split_once(',')?;
            s = if r.len() <= 2 {
                format!("{l}.{r}")
            } else {
                format!("{l}{r}")
            };
        }
        (None, Some(_)) => {
            let (l, r) = s.split_once('.')?;
            if r.len() > 2 {
                s = format!("{l}{r}");
            }
        }
        _ => (),
    }
    s.parse::<f64>().ok().map(|v| if negative { -v } else { v })
}
pub fn parse_number_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => parse_number(s),
        _ => None,
    }
}
pub fn overall_confidence(parts: &[&str]) -> &'static str {
    ["low", "medium", "high"]
        .into_iter()
        .find(|p| parts.contains(p))
        .unwrap_or("none")
}
pub fn implausible(r: Option<f64>, a: Option<f64>, p: Option<f64>) -> Vec<String> {
    let mut n = vec![];
    if let (Some(r), Some(p)) = (r, p) {
        if r > 0. && p.abs() > r {
            n.push("profit_exceeds_revenue".into());
        }
    }
    if let (Some(r), Some(a)) = (r, a) {
        if r > 0. && a > r * 10000. {
            n.push("revenue_negligible_against_assets".into());
        }
    }
    if let Some(r) = r {
        if (1990. ..=2100.).contains(&r) && r.fract() == 0. {
            n.push("revenue_looks_like_a_year".into());
        }
    }
    n
}
fn metric(lines: &[&str], keys: &[&str]) -> (Option<f64>, &'static str) {
    let re = Regex::new(r"\(?-?\d[\d\s.,]{0,20}\)?").unwrap();
    let numbers = |s: &str| {
        re.find_iter(s)
            .filter_map(|m| parse_number(m.as_str()))
            .collect::<Vec<_>>()
    };
    let (mut best, mut score) = (None, -1);
    for (i, line) in lines.iter().enumerate() {
        if !keys.iter().any(|k| normalize_text(line).contains(k)) {
            continue;
        }
        let mut ns = numbers(line);
        let mut base = 6;
        if ns.is_empty() && i + 1 < lines.len() {
            ns = numbers(lines[i + 1]);
            if !ns.is_empty() {
                base += 2;
            }
        }
        for n in ns {
            let s = base
                + if n.abs() >= 1000000. {
                    5
                } else if n.abs() >= 100000. {
                    3
                } else if n.abs() >= 10000. {
                    2
                } else {
                    1
                };
            if s > score {
                score = s;
                best = Some(n);
            }
        }
    }
    (
        best,
        if best.is_none() {
            "none"
        } else if score >= 12 {
            "high"
        } else if score >= 8 {
            "medium"
        } else {
            "low"
        },
    )
}
pub fn parse_financials_from_text(text: &str, year: Option<i32>, mut notes: Vec<String>) -> Value {
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let (r, rc) = metric(&lines, &["trzby", "obrat", "vykony", "i."]);
    let (a, ac) = metric(&lines, &["aktiva celkem", "aktiva", "b."]);
    let (p, pc) = metric(
        &lines,
        &["vysledek hospodareni", "zisk", "ztrata", "hospodareni"],
    );
    let failed = implausible(r, a, p);
    let confidence = if failed.is_empty() {
        overall_confidence(&[rc, ac, pc])
    } else {
        "low"
    };
    notes.extend(failed);
    json!({"year":year.filter(|y| *y != 0).or_else(||extract_year(text)),"revenue":r,"totalAssets":a,"netProfit":p,"confidence":confidence,"sourceNotes":notes})
}
pub fn score(path: &str, metric: usize) -> i32 {
    let p = normalize_text(path);
    let keys: &[(&str, i32)] = match metric {
        0 => &[("trzby", 12), ("obrat", 10), ("i.", 3)],
        1 => &[("aktivacelkem", 14), ("aktiva", 9), ("b.", 3)],
        _ => &[("vysledekhospodareni", 14), ("hospodareni", 8)],
    };
    keys.iter()
        .filter(|(k, _)| p.contains(k))
        .map(|(_, s)| s)
        .sum::<i32>()
        + if metric == 2 && (p.contains("zisk") || p.contains("ztrata")) {
            7
        } else {
            0
        }
}
pub fn collect_numeric_nodes(xml: &str) -> Option<Vec<(String, f64)>> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    fn walk(n: roxmltree::Node<'_, '_>, parent: &str, out: &mut Vec<(String, f64)>) {
        let path = if parent.is_empty() {
            n.tag_name().name().to_owned()
        } else {
            format!("{parent}.{}", n.tag_name().name())
        };
        if let Some(t) = n
            .first_child()
            .filter(|c| c.is_text())
            .and_then(|c| c.text())
        {
            if let Some(v) = parse_number(t) {
                out.push((path.clone(), v));
            }
        }
        for c in n.children().filter(|c| c.is_element()) {
            walk(c, &path, out);
        }
    }
    let mut out = vec![];
    walk(doc.root_element(), "", &mut out);
    Some(out)
}
pub fn pick_best(nodes: &[(String, f64)], metric: usize) -> Option<f64> {
    let (mut best, mut highest) = (None, 0);
    for (p, v) in nodes {
        let s = score(p, metric);
        if s > highest {
            highest = s;
            best = Some(*v);
        }
    }
    best
}
pub fn parse_financial_xml(xml: &str, year: Option<i32>) -> Option<Value> {
    let nodes = collect_numeric_nodes(xml)?;
    Some(
        json!({"year":year.filter(|y| *y != 0).or_else(||extract_year(xml)),"revenue":pick_best(&nodes,0),"totalAssets":pick_best(&nodes,1),"netProfit":pick_best(&nodes,2)}),
    )
}
pub fn parse_financial_xhtml(html: &str, year: Option<i32>) -> Value {
    let doc = scraper::Html::parse_document(html);
    let text = doc
        .root_element()
        .descendants()
        .filter(|node| {
            !node
                .ancestors()
                .filter_map(scraper::ElementRef::wrap)
                .any(|e| matches!(e.value().name(), "script" | "style" | "template"))
        })
        .filter_map(|node| node.value().as_text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        return json!({"year":year,"revenue":null,"totalAssets":null,"netProfit":null,"confidence":"none","sourceNotes":["xhtml_empty_content"]});
    }
    parse_financials_from_text(&text, year, vec!["parsed_from_xhtml".into()])
}
pub fn extract_pdf_text(bytes: &[u8], pages: Option<usize>) -> Result<String, crate::Error> {
    if let Some(limit) = pages {
        Ok(pdf_extract::extract_text_from_mem_by_pages(bytes)?
            .into_iter()
            .take(limit)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n"))
    } else {
        Ok(pdf_extract::extract_text_from_mem_by_pages(bytes)?
            .into_iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"))
    }
}
pub fn parse_financial_pdf(bytes: &[u8], year: Option<i32>) -> Value {
    let mut notes = vec![];
    let mut text = match extract_pdf_text(bytes, Some(8)) {
        Ok(t) => {
            notes.push("pdf_text_layer_used".into());
            t
        }
        Err(_) => {
            notes.push("pdf_text_extraction_failed".into());
            String::new()
        }
    };
    if text.trim().chars().count() < 120 {
        match ocr_pdf(bytes) {
            Ok(t) => {
                if !t.is_empty() {
                    notes.push("ocr_used".into());
                    text = format!("{text}\n{t}").trim().into();
                }
            }
            Err(n) => notes.push(n.into()),
        }
    }
    if text.trim().is_empty() {
        json!({"year":year,"revenue":null,"totalAssets":null,"netProfit":null,"confidence":"none","sourceNotes":notes})
    } else {
        parse_financials_from_text(&text, year, notes)
    }
}
fn ocr_pdf(bytes: &[u8]) -> Result<String, &'static str> {
    use std::{fs, process::Command};
    if !Command::new("tesseract")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Err("tesseract_binary_unavailable");
    }
    let dir = tempfile::tempdir().map_err(|_| "ocr_failed")?;
    let input = dir.path().join("input.pdf");
    fs::write(&input, bytes).map_err(|_| "ocr_failed")?;
    let prefix = dir.path().join("page");
    let result = Command::new("pdftoppm")
        .args(["-f", "1", "-l", "5", "-r", "144", "-png"])
        .arg(input)
        .arg(&prefix)
        .output()
        .map_err(|_| "ocr_dependencies_unavailable")?;
    if !result.status.success() {
        return Err("ocr_failed");
    }
    let mut files = fs::read_dir(dir.path())
        .map_err(|_| "ocr_failed")?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect::<Vec<_>>();
    files.sort();
    let mut chunks = vec![];
    for file in files {
        let output = Command::new("tesseract")
            .arg(file)
            .args(["stdout", "-l", "ces+eng"])
            .output()
            .map_err(|_| "ocr_failed")?;
        if !output.status.success() {
            return Err("ocr_failed");
        }
        let s = String::from_utf8_lossy(&output.stdout).into_owned();
        if !s.trim().is_empty() {
            chunks.push(s);
        }
    }
    Ok(chunks.join("\n"))
}
