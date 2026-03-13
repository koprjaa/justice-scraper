#
# Project: justice-scraper
# File:    config.py
#
# Description:
# URL templates, timeouts, rate limits, and feature flags for the justice fetcher.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

JUSTICE_ORIGIN = "https://or.justice.cz"

STEP1_URL_PRIMARY = (
    "https://or.justice.cz/ias/ui/rejstrik-$firma"
    "?ico={ico}&jenPlatne=PLATNE&polozek=50&typHledani=STARTS_WITH"
)
STEP1_URL_FALLBACK = "https://or.justice.cz/ias/ui/rejstrik-firma?ico={ico}"
STEP2_URL = "https://or.justice.cz/ias/ui/vypis-sl-firma?subjektId={subjekt_id}"
STEP3_URL = "https://or.justice.cz/ias/ui/vypis-sl-detail?dokument={dokument_id}"

# Step1/2/3 HTML navigation (optimized for speed)
REQUEST_TIMEOUT_SECONDS = 8
MAX_RETRIES = 1
RETRY_BACKOFF_SECONDS = 0.2

# Attachments on CDN: short timeout, no retries
ATTACHMENT_TIMEOUT_SECONDS = 12
ATTACHMENT_MAX_RETRIES = 0

# Navigation only; attachments bypass
RATE_LIMIT_SECONDS = 0.05

MAX_DOCUMENT_CANDIDATES = 3

# Below this, fall back to OCR for PDF
OCR_MIN_TEXT_CHARS = 120

# Rotate to reduce blocking
USER_AGENTS: list[str] = [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:126.0) Gecko/20100101 Firefox/126.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0",
]
