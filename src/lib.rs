// Project: justice-scraper
// File: src/lib.rs
// Description: Financial statement parsers and registry client.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
pub mod html;
pub mod http;
pub mod jobs;
pub mod parsers;
pub mod service;
pub use service::fetch_justice_financials;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
