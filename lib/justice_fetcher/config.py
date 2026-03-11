# Project: justice-scraper
# File: config.py
# Description: URL templates, timeouts, rate limits, and feature flags for the justice fetcher.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

JUSTICE_ORIGIN = "https://or.justice.cz"

STEP1_URL_PRIMARY = "https://or.justice.cz/ias/ui/rejstrik-firma?ico={ico}"
STEP1_URL_FALLBACK = "https://or.justice.cz/ias/ui/rejstrik-$firma?ico={ico}"
STEP2_URL = "https://or.justice.cz/ias/ui/vypis-sl-firma?subjektId={subjekt_id}"
STEP3_URL = "https://or.justice.cz/ias/ui/vypis-sl-detail?dokument={dokument_id}"

# Navigation (step1/2/3 HTML)
REQUEST_TIMEOUT_SECONDS = 15
MAX_RETRIES = 2
RETRY_BACKOFF_SECONDS = 0.5

# Attachments: CDN is slow; long timeout, no retries
ATTACHMENT_TIMEOUT_SECONDS = 60
ATTACHMENT_MAX_RETRIES = 0

# Rate limit for navigation only; attachments bypass
RATE_LIMIT_SECONDS = 0.5

MAX_DOCUMENT_CANDIDATES = 3

# Below this, PDF text layer is treated as missing and OCR is used
OCR_MIN_TEXT_CHARS = 120

# Rotate to reduce blocking risk
USER_AGENTS: list[str] = [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:126.0) Gecko/20100101 Firefox/126.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0",
]
