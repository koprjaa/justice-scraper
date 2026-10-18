// Project: justice-scraper
// File: src/service.rs
// Description: Three-step financial statement retrieval.
// Author: Jan Alexandr Kopřiva
// License: MIT
use crate::{
    html::*,
    http::{log, JusticeHttpClient},
    parsers::*,
};
use chrono::{Datelike, Utc};
use serde_json::{json, Value};
fn result(ico: &str, id: &str, records: Vec<Value>) -> Value {
    json!({"ico":ico,"subjektId":id,"financials":records,"lastUpdated":Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros,false)})
}
pub async fn fetch_justice_financials(ico: &str) -> Result<Value, crate::Error> {
    let client = JusticeHttpClient::new(ORIGIN)?;
    Ok(fetch_with_client(&client, ico).await)
}
pub async fn fetch_with_client(client: &JusticeHttpClient, raw: &str) -> Value {
    let ico = regex::Regex::new(r"\D")
        .unwrap()
        .replace_all(raw, "")
        .chars()
        .take(8)
        .collect::<String>();
    if ico.is_empty() {
        log(
            "INFO",
            &format!("justice-scraper ico={raw} status=invalid_ico"),
        );
        return result(raw, "", vec![]);
    }
    log("INFO", &format!("justice-scraper ico={ico} status=start"));
    let origin = &client.origin;
    let variants=[("step1_primary",format!("{origin}/ias/ui/rejstrik-$firma?ico={ico}&jenPlatne=PLATNE&polozek=50&typHledani=STARTS_WITH")),("step1_fallback",format!("{origin}/ias/ui/rejstrik-firma?ico={ico}"))];
    let mut html = None;
    let mut id = None;
    for (step, url) in variants {
        html = client.text(&url, &ico, step, None).await;
        if let Some(ref h) = html {
            id = extract_subjekt_id(h);
            if id.is_some() {
                break;
            }
        }
    }
    let Some(id) = id else {
        log(
            "INFO",
            &format!(
                "justice-scraper ico={ico} status={}",
                if html.is_none() {
                    "step1_failed"
                } else {
                    "subjekt_not_found"
                }
            ),
        );
        return result(&ico, "", vec![]);
    };
    let Some(list) = client
        .text(
            &format!("{origin}/ias/ui/vypis-sl-firma?subjektId={id}"),
            &ico,
            "step2_documents",
            None,
        )
        .await
    else {
        log(
            "INFO",
            &format!("justice-scraper ico={ico} subjektId={id} status=documents_failed"),
        );
        return result(&ico, &id, vec![]);
    };
    let candidates = parse_document_candidates(&list, origin);
    if candidates.is_empty() {
        log(
            "INFO",
            &format!("justice-scraper ico={ico} subjektId={id} status=no_financial_documents"),
        );
        return result(&ico, &id, vec![]);
    }
    let mut records = vec![];
    for c in candidates {
        records.push(process_candidate(client, &ico, &c).await);
    }
    records.sort_by_key(|r| std::cmp::Reverse(r["year"].as_i64().unwrap_or(0)));
    log(
        "INFO",
        &format!(
            "justice-scraper ico={ico} subjektId={id} status=done records={}",
            records.len()
        ),
    );
    result(&ico, &id, records)
}
async fn process_candidate(client: &JusticeHttpClient, ico: &str, c: &Candidate) -> Value {
    let year = c.year.unwrap_or_else(|| Utc::now().year());
    let fallback = |note: &str| json!({"year":year,"revenue":null,"totalAssets":null,"netProfit":null,"sourceType":"xhtml","documentUrl":c.detail_url,"dokumentId":c.dokument_id,"parserUsed":"fallback","confidence":"none","sourceNotes":[note]});
    let Some(detail) = client
        .text(
            &c.detail_url,
            ico,
            &format!("step3_detail_{}", c.dokument_id),
            None,
        )
        .await
    else {
        return fallback("document_detail_unavailable");
    };
    let urls = extract_attachment_urls(&detail, Some(&c.dokument_id), &client.origin);
    for (kind, url) in urls.into_iter().enumerate() {
        let Some(url) = url else { continue };
        let source = ["xml", "xhtml", "pdf"][kind];
        let mut parsed = if kind == 2 {
            json!({"year":year,"revenue":null,"totalAssets":null,"netProfit":null,"confidence":"none","sourceNotes":["pdf_url_collected_not_parsed"]})
        } else {
            let Some(text) = client
                .text(
                    &url,
                    ico,
                    &format!("step3_{source}_{}", c.dokument_id),
                    Some(&c.detail_url),
                )
                .await
            else {
                continue;
            };
            if kind == 0 {
                let Some(mut p) = parse_financial_xml(&text, c.year) else {
                    continue;
                };
                p["confidence"] = json!("high");
                p["sourceNotes"] = json!(["parsed_from_xml"]);
                p
            } else {
                parse_financial_xhtml(&text, c.year)
            }
        };
        if parsed["year"].is_null() {
            parsed["year"] = json!(year);
        }
        parsed["sourceType"] = json!(source);
        parsed["documentUrl"] = json!(url);
        parsed["dokumentId"] = json!(c.dokument_id);
        parsed["parserUsed"] = json!(if kind == 2 { "url_only" } else { source });
        return json!({
            "year": parsed["year"], "revenue": parsed["revenue"],
            "totalAssets": parsed["totalAssets"], "netProfit": parsed["netProfit"],
            "sourceType": parsed["sourceType"], "documentUrl": parsed["documentUrl"],
            "dokumentId": parsed["dokumentId"], "parserUsed": parsed["parserUsed"],
            "confidence": parsed["confidence"], "sourceNotes": parsed["sourceNotes"]
        });
    }
    fallback("no_xml_xhtml_or_pdf_attachment_found")
}
