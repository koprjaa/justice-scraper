// Project: justice-scraper
// File: tests/network.rs
// Description: Offline registry workflow, retry, and CSV smoke tests.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use justice_scraper::{html::*, http::JusticeHttpClient, service::fetch_with_client};
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};
const LIST: &str = include_str!("fixtures/listing.html");
const DETAIL: &str = include_str!("fixtures/detail.html");
#[test]
fn html_parsers() {
    let (ico, url, date) = parse_main_page(LIST);
    assert_eq!(ico.as_deref(), Some("27074358"));
    assert_eq!(
        url.as_deref(),
        Some("https://or.justice.cz/ias/ui/vypis-sl-detail?dokument=42")
    );
    assert_eq!(date.as_deref(), Some("31.12.2023"));
    assert_eq!(
        parse_document_link(DETAIL),
        Some(("https://or.justice.cz/download/42.xml".into(), "XML".into()))
    );
    assert_eq!(
        extract_subjekt_id("<a href='./vypis-sl-firma?subjektId=7&amp;x=1'>x</a>").as_deref(),
        Some("7")
    );
    assert_eq!(extract_subjekt_id("SUBJEKTID=99 ").as_deref(), Some("99"));
    assert_eq!(normalize_url("//example.test/a"), "https://example.test/a");
    assert_eq!(normalize_url("/a"), "https://or.justice.cz/a");
    let c = parse_document_candidates(LIST, ORIGIN);
    assert_eq!(c[0].year, Some(2023));
    let urls = extract_attachment_urls(DETAIL, Some("42"), ORIGIN);
    assert_eq!(
        urls[0].as_deref(),
        Some("https://or.justice.cz/download/42.xml")
    );
    assert!(urls[1].is_none());
    assert!(urls[2].is_some());
    assert!(score_attachment(2, "/GDPR.pdf".to_lowercase().as_str(), "", Some("42")) < 0);
}
#[tokio::test]
async fn financial_workflow() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ias/ui/rejstrik-$firma"))
        .and(query_param("ico", "27074358"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<a href='./vypis-sl-firma?subjektId=7'>company</a>"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-detail"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DETAIL))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/download/42.xml"))
        .and(header(
            "Referer",
            format!("{}/ias/ui/vypis-sl-detail?dokument=42", server.uri()),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/statement.xml")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = JusticeHttpClient::new(&server.uri()).unwrap();
    let r = fetch_with_client(&c, "CZ27074358").await;
    assert_eq!(r["ico"], "27074358");
    assert_eq!(r["subjektId"], "7");
    assert_eq!(r["financials"][0]["revenue"], 15000000.);
    assert_eq!(r["financials"][0]["confidence"], "high");
    assert!(r["lastUpdated"].as_str().unwrap().ends_with("+00:00"));
}
#[tokio::test]
async fn retry_and_attachment_policy() {
    let s = MockServer::start().await;
    Mock::given(path("/error"))
        .respond_with(ResponseTemplate::new(503))
        .expect(2)
        .mount(&s)
        .await;
    Mock::given(path("/attachment"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/html"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>blocked</html>"))
        .expect(1)
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    assert!(c
        .text(&format!("{}/error", s.uri()), "1", "step1_primary", None)
        .await
        .is_none());
    assert!(c
        .text(&format!("{}/attachment", s.uri()), "1", "step3_xml_1", None)
        .await
        .is_none());
    assert!(c
        .text(
            &format!("{}/missing", s.uri()),
            "1",
            "step2_documents",
            None
        )
        .await
        .is_none());
    assert!(c
        .fetch(&format!("{}/html", s.uri()), "1", "step3_pdf_1", None, true)
        .await
        .is_none());
}
#[tokio::test]
async fn range_smoke_and_csv_bytes() {
    let s = MockServer::start().await;
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST))
        .expect(10)
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-detail"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DETAIL))
        .expect(10)
        .mount(&s)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("document_links.csv");
    let count = justice_scraper::jobs::scrape_range(1, 10, &out, &s.uri())
        .await
        .unwrap();
    assert_eq!(count, 10);
    let bytes = std::fs::read(out).unwrap();
    assert!(bytes.starts_with(b"ico,vznik_listiny,link,typ\r\n"));
    assert_eq!(
        String::from_utf8(bytes.clone()).unwrap().lines().count(),
        11
    );
    let expected = include_bytes!("fixtures/range.csv");
    let header_end = expected.iter().position(|b| *b == b'\n').unwrap() + 1;
    let mut full = expected[..header_end].to_vec();
    for _ in 0..10 {
        full.extend_from_slice(&expected[header_end..]);
    }
    assert_eq!(bytes, full);
}
#[tokio::test]
async fn invalid_ico_and_fallback_pdf() {
    let s = MockServer::start().await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    assert!(fetch_with_client(&c, "bad").await["financials"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(s.received_requests().await.unwrap().is_empty());
    Mock::given(path("/ias/ui/rejstrik-$firma"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/rejstrik-firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string("subjektId=7 "))
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST))
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-detail"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<a href='/42.pdf'>file.pdf</a>"))
        .mount(&s)
        .await;
    let r = fetch_with_client(&c, "123").await;
    assert_eq!(r["financials"][0]["parserUsed"], "url_only");
    assert_eq!(r["financials"][0]["sourceType"], "pdf");
}
#[tokio::test]
async fn downloader_session_flow_and_existing_files() {
    let s = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.csv");
    let texts = dir.path().join("texts");
    std::fs::write(&input, "ico,subjektId,dokumentId,year\n123,7,42,2023\n").unwrap();
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Set-Cookie", "session=ok; Path=/")
                .set_body_string(
                    "<a href='./vypis-sl-detail?dokument=42&amp;spis=token'>detail</a>",
                ),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-detail"))
        .and(query_param("spis", "token"))
        .and(header("cookie", "session=ok"))
        .and(header(
            "Referer",
            format!("{}/ias/ui/vypis-sl-firma?subjektId=7", s.uri()),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<a href='/content/download/42.pdf?x=1&amp;y=2'>download</a>"),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/content/download/42.pdf"))
        .and(query_param("y", "2"))
        .and(header(
            "Referer",
            format!("{}/ias/ui/vypis-sl-detail?dokument=42&spis=token", s.uri()),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes(include_bytes!("fixtures/statement.pdf")),
        )
        .expect(1)
        .mount(&s)
        .await;
    justice_scraper::jobs::download(&input, &texts, &s.uri())
        .await
        .unwrap();
    let out = texts.join("123_2023_42.txt");
    assert!(std::fs::read_to_string(&out)
        .unwrap()
        .contains("Revenue 15000000"));
    justice_scraper::jobs::download(&input, &texts, &s.uri())
        .await
        .unwrap();
}
#[tokio::test]
async fn batch_invalid_identifier_is_offline() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.csv");
    let output = dir.path().join("output.csv");
    std::fs::write(&input, "ico\ninvalid\n").unwrap();
    justice_scraper::jobs::batch(&input, &output).await.unwrap();
    assert_eq!(std::fs::read(output).unwrap(),b"ico,subjektId,dokumentId,year,documentUrl,sourceType,parserUsed,sourceNotes\r\ninvalid,,,,,,,no_records\r\n");
}
#[tokio::test]
async fn limiter_spaces_concurrent_requests() {
    let limiter = std::sync::Arc::new(justice_scraper::http::RateLimiter::new(0.05));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..3 {
        let limiter = limiter.clone();
        tasks.spawn(async move {
            limiter.wait_turn().await;
            std::time::Instant::now()
        });
    }
    let mut times = vec![];
    while let Some(t) = tasks.join_next().await {
        times.push(t.unwrap());
    }
    times.sort();
    for pair in times.windows(2) {
        assert!(pair[1].duration_since(pair[0]) >= std::time::Duration::from_millis(45));
    }
}
#[tokio::test]
async fn unsafe_download_names_are_rejected_before_http() {
    let s = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.csv");
    std::fs::write(&input,"ico,subjektId,dokumentId,year\n../../escape,7,42,2023\n123,7,../42,2023\n123,7,42,../2023\n").unwrap();
    justice_scraper::jobs::download(&input, &dir.path().join("texts"), &s.uri())
        .await
        .unwrap();
    assert!(s.received_requests().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(dir.path().join("texts")).unwrap().count(),
        0
    );
}
#[tokio::test]
async fn xml_failure_uses_xhtml() {
    let s = MockServer::start().await;
    Mock::given(path("/ias/ui/rejstrik-$firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string("subjektId=7 "))
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST))
        .mount(&s)
        .await;
    let detail = Mock::given(path("/ias/ui/vypis-sl-detail")).respond_with(
        ResponseTemplate::new(200)
            .set_body_string("<a href='/42.xml'>statement.xml</a><a href='/42.xhtml'>XHTML</a>"),
    );
    detail.mount(&s).await;
    Mock::given(path("/42.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<broken>"))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/42.xhtml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<p>Tržby</p><p>15 000 000</p><script>Tržby 999999999</script>"),
        )
        .expect(1)
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    let r = fetch_with_client(&c, "123").await;
    assert_eq!(r["financials"][0]["parserUsed"], "xhtml");
    assert_eq!(r["financials"][0]["revenue"], 15000000.);
}
#[test]
fn candidate_order_and_attachment_scope() {
    let html="<table><tr><td><a href='?dokument=1'>účetní závěrka [2022]</a></td></tr><tr><td><a href='?dokument=2'>účetní závěrka [2023]</a></td></tr><tr><td><a href='?dokument=42'>účetní závěrka [2023]</a></td></tr></table>";
    assert_eq!(parse_document_candidates(html, ORIGIN)[0].dokument_id, "42");
    let html="<table><tr><th>Digitální podoba:</th></tr><tr><td><a href='42.pdf'>statement.pdf</a></td></tr><tr><th>Other section</th></tr><tr><td><a href='42.xml'>statement.xml</a></td></tr></table>";
    let urls = extract_attachment_urls(html, Some("42"), ORIGIN);
    assert!(urls[0].is_none());
    assert!(urls[2].is_some());
}
#[tokio::test]
async fn navigation_timeout_is_enforced_and_retried() {
    let s = MockServer::start().await;
    Mock::given(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(60)))
        .expect(2)
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        c.text(&format!("{}/slow", s.uri()), "1", "step1_primary", None),
    )
    .await;
    assert!(result.unwrap().is_none());
}
#[tokio::test]
async fn declared_legacy_charset_matches_python() {
    let s = MockServer::start().await;
    Mock::given(path("/latin"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=windows-1250")
                .set_body_bytes(vec![b'T', b'r', 0x9e, b'b', b'y']),
        )
        .expect(1)
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    assert_eq!(
        c.text(&format!("{}/latin", s.uri()), "1", "step3_xml_1", None)
            .await
            .as_deref(),
        Some("Tržby")
    );
}
#[tokio::test]
async fn malformed_utf8_is_replaced_and_http_bom_is_removed() {
    let s = MockServer::start().await;
    for (p, bytes) in [("/bad", vec![0xff]), ("/bom", vec![0xef, 0xbb, 0xbf, b'x'])] {
        Mock::given(path(p))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/plain; charset=utf-8")
                    .set_body_bytes(bytes),
            )
            .expect(1)
            .mount(&s)
            .await;
    }
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    assert_eq!(
        c.text(&format!("{}/bad", s.uri()), "1", "step3_xml_1", None)
            .await
            .as_deref(),
        Some("\u{fffd}")
    );
    assert_eq!(
        c.text(&format!("{}/bom", s.uri()), "1", "step3_xml_1", None)
            .await
            .as_deref(),
        Some("x")
    );
}
#[test]
fn ico_is_padded_and_overlong_is_rejected() {
    use justice_scraper::service::normalize_ico;
    assert_eq!(normalize_ico("6947").as_deref(), Some("00006947"));
    assert_eq!(normalize_ico("27 07 43 58").as_deref(), Some("27074358"));
    assert_eq!(normalize_ico("123456789"), None);
    assert_eq!(normalize_ico("invalid"), None);
}
#[tokio::test]
async fn short_ico_is_queried_with_leading_zeros() {
    let s = MockServer::start().await;
    Mock::given(path("/ias/ui/rejstrik-$firma"))
        .and(query_param("ico", "00006947"))
        .respond_with(ResponseTemplate::new(200).set_body_string("no company here"))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/rejstrik-firma"))
        .and(query_param("ico", "00006947"))
        .respond_with(ResponseTemplate::new(200).set_body_string("no company here"))
        .expect(1)
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    let r = fetch_with_client(&c, "6947").await;
    assert_eq!(r["ico"], "00006947");
    assert!(r.get("failed").is_none());
}
#[tokio::test]
async fn failed_requests_are_not_recorded_as_no_records() {
    let s = MockServer::start().await;
    Mock::given(path("/ias/ui/rejstrik-$firma"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&s)
        .await;
    Mock::given(path("/ias/ui/rejstrik-firma"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&s)
        .await;
    let c = JusticeHttpClient::new(&s.uri()).unwrap();
    assert_eq!(
        fetch_with_client(&c, "27074358").await["failed"],
        "step1_failed"
    );
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, "ico\n27074358\n").unwrap();
    let err = justice_scraper::jobs::batch_from(&input, &output, &s.uri())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("1 companies failed"));
    // Only the header: the company stays unprocessed and is retried next time.
    assert_eq!(std::fs::read_to_string(&output).unwrap().lines().count(), 1);
}
#[tokio::test]
async fn batch_resume_matches_ico_however_it_was_typed() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, "ico\n27 07 43 58\n6947\n").unwrap();
    std::fs::write(
        &output,
        "ico,subjektId,dokumentId,year,documentUrl,sourceType,parserUsed,sourceNotes\r\n27074358,,,,,,,no_records\r\n00006947,,,,,,,no_records\r\n",
    )
    .unwrap();
    let before = std::fs::read(&output).unwrap();
    // Nothing is left to fetch, so no request is made (the origin is unreachable).
    justice_scraper::jobs::batch_from(&input, &output, "http://127.0.0.1:9")
        .await
        .unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), before);
}
#[tokio::test]
async fn failed_range_keeps_the_previous_output() {
    let s = MockServer::start().await;
    Mock::given(path("/ias/ui/vypis-sl-firma"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&s)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("document_links.csv");
    std::fs::write(&out, "ico,vznik_listiny,link,typ\r\n1,2,3,XML\r\n").unwrap();
    let err = justice_scraper::jobs::scrape_range(1, 1, &out, &s.uri())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("kept the existing"));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "ico,vznik_listiny,link,typ\r\n1,2,3,XML\r\n"
    );
}
