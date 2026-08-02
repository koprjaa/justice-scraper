#
# Project: justice-scraper
# File:    __init__.py
#
# Description:
# Package entry point. Exposes fetch_justice_financials.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

from .service import fetch_justice_financials

__all__ = ["fetch_justice_financials"]
