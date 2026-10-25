// Project: justice-scraper
// File: src/html.rs
// Description: Registry navigation and attachment selection.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use crate::parsers::{decimal_ascii, extract_year, normalize_text};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
pub const ORIGIN: &str = "https://or.justice.cz";
pub fn select(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
pub fn text(e: ElementRef<'_>, sep: &str) -> String {
    e.text()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}
pub fn join(base: &str, href: &str) -> String {
    reqwest::Url::parse(base)
        .and_then(|b| b.join(href))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| href.to_owned())
}
pub fn normalize_url(href: &str) -> String {
    if href.starts_with("//") {
        format!("https:{href}")
    } else if href.starts_with("http://") || href.starts_with("https://") {
        href.into()
    } else if let Some(h) = href.strip_prefix("./") {
        format!("{ORIGIN}/ias/ui/{h}")
    } else {
        format!("{ORIGIN}/{}", href.trim_start_matches('/'))
    }
}
pub fn extract_subjekt_id(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let re = Regex::new(r"(?i)[?&]subjektId=([^&]+)").unwrap();
    for a in doc.select(&select("a[href*='vypis-sl-firma?subjektId=']")) {
        if let Some(c) = re.captures(a.value().attr("href").unwrap_or("")) {
            return Some(c[1].into());
        }
    }
    Regex::new(r#"(?i)subjektId=([^"&\s]+)"#)
        .unwrap()
        .captures(html)
        .map(|c| c[1].into())
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub dokument_id: String,
    pub year: Option<i32>,
    pub detail_url: String,
}
pub fn parse_document_candidates(html: &str, origin: &str) -> Vec<Candidate> {
    let doc = Html::parse_document(html);
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    let re = Regex::new(r"(?i)[?&]dokument=([^&]+)").unwrap();
    for row in doc.select(&select("tr")) {
        if !normalize_text(
            &text(row, " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        )
        .contains("ucetni zaverka")
        {
            continue;
        }
        let link = row
            .select(&select("a[href*='vypis-sl-detail?dokument=']"))
            .next()
            .or_else(|| row.select(&select("a[href*='dokument=']")).next());
        if let Some(link) = link {
            let href = link.value().attr("href").unwrap_or("");
            if let Some(c) = re.captures(href) {
                let id = c[1].to_string();
                if seen.insert(id.clone()) {
                    out.push(Candidate {
                        dokument_id: id,
                        year: extract_year(&text(row, " ")),
                        detail_url: join(&format!("{origin}/ias/ui/"), href),
                    });
                }
            }
        }
    }
    fn integer_key(id: &str) -> (bool, String) {
        let id = id.trim_matches(crate::parsers::python_whitespace);
        let (negative, digits) = if let Some(s) = id.strip_prefix('-') {
            (true, s)
        } else {
            (false, id.strip_prefix('+').unwrap_or(id))
        };
        let groups: Vec<_> = digits.split('_').collect();
        let number = if groups.iter().any(|s| s.is_empty()) {
            None
        } else {
            decimal_ascii(&groups.concat())
        };
        let number = number.unwrap_or_else(|| "0".into());
        let number = number.trim_start_matches('0');
        (negative && !number.is_empty(), number.into())
    }
    out.sort_by(|a, b| {
        let (an, ad) = integer_key(&a.dokument_id);
        let (bn, bd) = integer_key(&b.dokument_id);
        let integers = bn.cmp(&an).then_with(|| {
            let order = ad.len().cmp(&bd.len()).then_with(|| ad.cmp(&bd));
            if an {
                order.reverse()
            } else {
                order
            }
        });
        b.year
            .unwrap_or(0)
            .cmp(&a.year.unwrap_or(0))
            .then(integers.reverse())
    });
    out.truncate(1);
    out
}
fn sibling_row<'a>(e: ElementRef<'a>) -> Option<ElementRef<'a>> {
    e.next_siblings()
        .filter_map(ElementRef::wrap)
        .find(|e| e.value().name() == "tr")
}
fn parent_row(e: ElementRef<'_>) -> Option<ElementRef<'_>> {
    e.ancestors()
        .filter_map(ElementRef::wrap)
        .find(|e| e.value().name() == "tr")
}
fn links(e: ElementRef<'_>) -> Vec<(String, String)> {
    e.select(&select("a[href]"))
        .filter_map(|a| {
            let h = a.value().attr("href")?.trim();
            if h.is_empty() {
                None
            } else {
                Some((
                    text(a, " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    h.into(),
                ))
            }
        })
        .collect()
}
pub fn extract_attachment_urls(html: &str, id: Option<&str>, origin: &str) -> [Option<String>; 3] {
    let doc = Html::parse_document(html);
    let mut all = vec![];
    if let Some(header) = doc
        .select(&select("th"))
        .find(|e| text(*e, "").contains("Digitální podoba:"))
    {
        let mut row = parent_row(header).and_then(sibling_row);
        for _ in 0..12 {
            let Some(r) = row else { break };
            if r.select(&select("th")).next().is_some() {
                break;
            }
            all.extend(links(r));
            row = sibling_row(r);
        }
    }
    if all.is_empty() {
        all = links(doc.root_element());
    }
    let mut seen = std::collections::HashSet::new();
    all.retain(|(_, h)| seen.insert(h.clone()));
    std::array::from_fn(|kind| {
        let mut best = 0;
        let mut url = None;
        for (t, h) in &all {
            let s = score_attachment(kind, &h.to_lowercase(), &normalize_text(t), id);
            if s > best {
                best = s;
                url = Some(join(&format!("{origin}/ias/ui/"), h));
            }
        }
        url
    })
}
pub fn score_attachment(kind: usize, h: &str, t: &str, id: Option<&str>) -> i32 {
    if h.contains("ilinklistener-htmlcontainer-logo") {
        return -100;
    }
    if ["gdpr", "ochrana", "informace"]
        .iter()
        .any(|x| h.contains(x) || t.contains(x))
    {
        return -60;
    }
    let mut s = 0;
    if h.contains("content/download") || h.contains("/download/") {
        s += 4;
    }
    if id.is_some_and(|i| h.contains(i)) {
        s += 10;
    }
    if ["zaverka", "ucetni", "rozvaha", "vysledovka", "vykaz"]
        .iter()
        .any(|x| t.contains(x))
    {
        s += 6;
    }
    let matches = match kind {
        0 => h.contains(".xml") || t.contains(" xml") || t.contains("xbrl"),
        1 => h.contains(".xhtml") || t.contains("xhtml"),
        _ => h.contains(".pdf") || t.contains(".pdf") || t.contains(" pdf"),
    };
    if matches {
        s + if kind == 2 { 18 } else { 20 }
    } else {
        0
    }
}
pub fn parse_main_page(html: &str) -> (Option<String>, Option<String>, Option<String>) {
    let doc = Html::parse_document(html);
    let ico = doc
        .select(&select("th"))
        .find(|e| text(*e, "").contains("Identifikační číslo:"))
        .and_then(parent_row)
        .and_then(|r| r.select(&select("span.nowrap")).next())
        .map(|e| text(e, "").replace(' ', ""));
    let re = Regex::new(r"(?i)z[áa]v[ěe]rka").unwrap();
    for symbol in doc.select(&select("span.symbol")) {
        if !re.is_match(&text(symbol, "")) {
            continue;
        }
        if let Some(row) = parent_row(symbol) {
            let cells = row.select(&select("td")).collect::<Vec<_>>();
            if cells.len() >= 3 {
                if let Some(a) = cells[0].select(&select("a[href]")).next() {
                    return (
                        ico,
                        Some(normalize_url(a.value().attr("href").unwrap().trim())),
                        Some(text(cells[2], "")),
                    );
                }
            }
        }
    }
    (ico, None, None)
}
pub fn parse_document_link(html: &str) -> Option<(String, String)> {
    let doc = Html::parse_document(html);
    let header = doc
        .select(&select("th"))
        .find(|e| text(*e, "").contains("Digitální podoba:"))?;
    let mut row = parent_row(header).and_then(sibling_row);
    let (mut xml, mut pdf) = (None, None);
    for _ in 0..10 {
        let Some(r) = row else { break };
        for (t, h) in links(r) {
            let t = t.to_lowercase();
            let lower = h.to_lowercase();
            if xml.is_none() && (t.contains(".xml") || lower.contains(".xml")) {
                xml = Some(normalize_url(&h));
            }
            if pdf.is_none() && (t.contains(".pdf") || lower.contains(".pdf")) {
                pdf = Some(normalize_url(&h));
            }
        }
        row = sibling_row(r);
    }
    xml.map(|u| (u, "XML".into()))
        .or_else(|| pdf.map(|u| (u, "PDF".into())))
}
