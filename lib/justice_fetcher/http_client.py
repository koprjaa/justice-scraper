#
# Project: justice-scraper
# File:    http_client.py
#
# Description:
# HTTP client with rate limiting and attachment-specific timeout/retry for justice.cz.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import asyncio
import random

import aiohttp

from .config import (
    ATTACHMENT_MAX_RETRIES,
    ATTACHMENT_TIMEOUT_SECONDS,
    JUSTICE_ORIGIN,
    MAX_RETRIES,
    RETRY_BACKOFF_SECONDS,
    USER_AGENTS,
)
from .logging_utils import log_status
from .rate_limiter import AsyncRateLimiter

# Attachments on CDN: no rate limit, separate timeout
_ATTACHMENT_PREFIXES = ("step3_xml_", "step3_xhtml_", "step3_pdf_")


def _is_attachment_step(step: str) -> bool:
    return any(step.startswith(prefix) for prefix in _ATTACHMENT_PREFIXES)


class JusticeHttpClient:
    def __init__(self, session: aiohttp.ClientSession, limiter: AsyncRateLimiter) -> None:
        self._session = session
        self._limiter = limiter

    async def fetch_text(
        self,
        url: str,
        ico: str,
        step: str,
        referer: str | None = None,
    ) -> str | None:
        data = await self._fetch(url, ico, step, mode="text", referer=referer)
        return data if isinstance(data, str) else None

    async def fetch_bytes(
        self,
        url: str,
        ico: str,
        step: str,
        referer: str | None = None,
    ) -> bytes | None:
        data = await self._fetch(url, ico, step, mode="bytes", referer=referer)
        return data if isinstance(data, bytes) else None

    async def _fetch(
        self,
        url: str,
        ico: str,
        step: str,
        mode: str,
        referer: str | None = None,
    ) -> str | bytes | None:
        attachment = _is_attachment_step(step)
        max_retries = ATTACHMENT_MAX_RETRIES if attachment else MAX_RETRIES
        timeout = (
            aiohttp.ClientTimeout(total=ATTACHMENT_TIMEOUT_SECONDS)
            if attachment
            else None
        )
        headers = {
            "User-Agent": random.choice(USER_AGENTS),
            "Referer": referer or f"{JUSTICE_ORIGIN}/ias/ui/",  # required or server returns HTML instead of file
        }

        for attempt in range(max_retries + 1):
            try:
                if not attachment:
                    await self._limiter.wait_turn()

                async with self._session.get(
                    url,
                    headers=headers,
                    allow_redirects=True,
                    timeout=timeout,
                ) as response:
                    if 200 <= response.status < 300:
                        if mode == "bytes":
                            raw = await response.read()
                            content_type = response.headers.get("Content-Type", "")
                            if raw[:5] in (b"<!DOC", b"<html", b"<HTML") or "text/html" in content_type:
                                log_status(
                                    "justice-scraper ico=%s status=html_instead_of_file step=%s",
                                    ico,
                                    step,
                                )
                                return None
                            return raw
                        return await response.text()

                    log_status(
                        "justice-scraper ico=%s status=http_%s step=%s attempt=%s",
                        ico,
                        response.status,
                        step,
                        attempt + 1,
                    )
                    if 500 <= response.status <= 599 and attempt < max_retries:
                        await asyncio.sleep(RETRY_BACKOFF_SECONDS * (attempt + 1))
                        continue
                    return None

            except (aiohttp.ClientError, asyncio.TimeoutError) as exc:
                log_status(
                    "justice-scraper ico=%s status=error step=%s error=%s attempt=%s",
                    ico,
                    step,
                    exc.__class__.__name__,
                    attempt + 1,
                )
                if attempt < max_retries:
                    await asyncio.sleep(RETRY_BACKOFF_SECONDS * (attempt + 1))
                    continue
                return None

        return None  # pragma: no cover
