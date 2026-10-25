// Project: justice-scraper
// File: src/service.rs
// Description: Three-step financial statement retrieval.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
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
/// Registry number as the registry stores it: eight digits, leading zeros kept.
/// None when the input has no digits or more than eight (not a company number).
pub fn normalize_ico(raw: &str) -> Option<String> {
    let digits = regex::Regex::new(r"\D").unwrap().replace_all(raw, "");
    (!digits.is_empty() && digits.len() <= 8).then(|| format!("{digits:0>8}"))
}
pub async fn fetch_justice_financials(ico: &str) -> Result<Value, crate::Error> {
    fetch_justice_financials_from(ORIGIN, ico).await
}
pub async fn fetch_justice_financials_from(origin: &str, ico: &str) -> Result<Value, crate::Error> {
    let client = JusticeHttpClient::new(origin)?;
    Ok(fetch_with_client(&client, ico).await)
}
/// Marks a result that came from a failed request, so a batch can retry it
/// instead of recording "no_records" for good.
fn failed(mut result: Value, why: &str) -> Value {
    result["failed"] = json!(why);
    result
}
pub async fn fetch_with_client(client: &JusticeHttpClient, raw: &str) -> Value {
    let Some(ico) = normalize_ico(raw) else {
        log(
            "INFO",
            &format!("justice-scraper ico={raw} status=invalid_ico"),
        );
        return result(raw, "", vec![]);
    };
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
        let empty = result(&ico, "", vec![]);
        return if html.is_none() {
            failed(empty, "step1_failed")
        } else {
            empty
        };
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
        return failed(result(&ico, &id, vec![]), "documents_failed");
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
    let unavailable = records.iter().any(|r| {
        r["sourceNotes"]
            .as_array()
            .is_some_and(|n| n.iter().any(|n| n == "document_detail_unavailable"))
    });
    let done = result(&ico, &id, records);
    if unavailable {
        failed(done, "document_detail_unavailable")
    } else {
        done
    }
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
