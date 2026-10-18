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
#[test]
fn leading_blank_header_matches_python() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PO_ALL.csv"), "\nico\ninvalid\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let data = std::fs::read_to_string(dir.path().join("financials_links.csv")).unwrap();
    assert_eq!(data.lines().count(), 3);
    assert!(data.contains("ico,,,,,,,no_records"));
    assert!(data.contains("invalid,,,,,,,no_records"));
    std::fs::write(
        dir.path().join("financials_links.csv"),
        "\nico,subjektId,dokumentId,year\ninvalid,7,42,2023\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_download_pdfs"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}
#[test]
fn empty_and_bom_csv_inputs() {
    for input in [
        "",
        "ico\r\n",
        "\u{feff}ico\r\ninvalid\r\n",
        "ico\r\ninvalid_é\r\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("PO_ALL.csv"), input).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let output = dir.path().join("financials_links.csv");
        if input.contains("invalid") {
            let bytes = std::fs::read(output).unwrap();
            assert!(bytes.starts_with(
                b"ico,subjektId,dokumentId,year,documentUrl,sourceType,parserUsed,sourceNotes\r\n"
            ));
            assert!(String::from_utf8(bytes).unwrap().contains("no_records\r\n"));
        } else {
            assert!(!output.exists());
        }
    }
}
#[test]
fn oversized_csv_field_and_corrupt_resume_policy() {
    let dir = tempfile::tempdir().unwrap();
    let ico = "x".repeat(131073);
    std::fs::write(dir.path().join("PO_ALL.csv"), format!("ico\n{ico}\n")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let output = dir.path().join("financials_links.csv");
    assert!(std::fs::read_to_string(&output).unwrap().contains(&ico));
    std::fs::write(output, b"ico\ninvalid\n\xff\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("UTF-8"));
}
#[test]
fn embedded_csv_newlines_match_python_translation() {
    for newline in ["\r", "\r\n", "\n"] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("PO_ALL.csv"),
            format!("ico\r\n\"invalid{newline}company\"\r\n"),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_batch_scraper"))
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(std::fs::read(dir.path().join("financials_links.csv")).unwrap(), b"ico,subjektId,dokumentId,year,documentUrl,sourceType,parserUsed,sourceNotes\r\n\"invalid\ncompany\",,,,,,,no_records\r\n");
    }
}
