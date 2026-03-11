#
# Project: justice-scraper
# File:    rate_limiter.py
#
# Description:
# Async rate limiter for spacing navigation requests to justice.cz.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import asyncio
import time


class AsyncRateLimiter:
    def __init__(self, min_interval_seconds: float) -> None:
        self._min_interval_seconds = min_interval_seconds
        self._lock = asyncio.Lock()
        self._last_request_ts = 0.0

    async def wait_turn(self) -> None:
        async with self._lock:
            elapsed = time.monotonic() - self._last_request_ts
            if elapsed < self._min_interval_seconds:
                await asyncio.sleep(self._min_interval_seconds - elapsed)
            self._last_request_ts = time.monotonic()
