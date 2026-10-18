// Project: justice-scraper
// File: tests/cli.rs
// Description: CLI defaults, exit status, help, and resume behavior.
// Author: Jan Alexandr Kopřiva
// License: MIT
use std::process::Command;
#[test]
fn help() {
    for bin in [
        env!("CARGO_BIN_EXE_justice_scraper"),
        env!("CARGO_BIN_EXE_batch_scraper"),
        env!("CARGO_BIN_EXE_download_pdfs"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let out = Command::new(bin)
            .arg("--help")
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8(out.stdout).unwrap().contains("Usage:"));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
#[test]
fn missing_input_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let out = Command::new(env!("CARGO_BIN_EXE_download_pdfs"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().contains("not found"));
    assert!(dir.path().join("texts").is_dir());
}
#[test]
fn batch_resume_and_empty_download() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PO_ALL.csv"), "ico\n123\n").unwrap();
    let original=b"ico,subjektId,dokumentId,year,documentUrl,sourceType,parserUsed,sourceNotes\r\n123,,,,,,,no_records\r\n";
    std::fs::write(dir.path().join("financials_links.csv"), original).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("Already processed: 1. Remaining: 0"));
    assert_eq!(
        std::fs::read(dir.path().join("financials_links.csv")).unwrap(),
        original
    );
    let out = Command::new(env!("CARGO_BIN_EXE_download_pdfs"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "Found 0 documents to download."
    );
}
