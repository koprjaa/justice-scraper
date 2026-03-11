# Project: justice-scraper
# File: logging_utils.py
# Description: Status logging and default console handler for the fetcher.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

import logging

_logger = logging.getLogger("justice_fetcher")

# Console fallback when caller does not configure logging
if not logging.root.handlers:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )


def log_status(message: str, *args: object) -> None:
    _logger.info(message, *args)
