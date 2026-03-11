#!/usr/bin/env python3
# Project: justice-scraper
# File: test_scraper.py
# Description: Smoke test for the scraper over a small ID range.
# Author: Jan Alexandr Kopřiva jan.alexandr.kopriva@gmail.com
# License: Proprietary

import asyncio
import sys
from pathlib import Path

# Allow importing justice_scraper when run from project root
sys.path.insert(0, str(Path(__file__).parent))

from justice_scraper import scrape_range, OUTPUT_CSV


async def test_scraper():
    """Runs scrape_range(1, 10) and prints CSV summary."""
    print("=" * 60)
    print("TEST JUSTICE SCRAPER")
    print("=" * 60)
    print("ID range: 1-10")
    print(f"Output file: {OUTPUT_CSV}")
    print()

    if OUTPUT_CSV.exists():
        OUTPUT_CSV.unlink()
        print(f"Removed existing {OUTPUT_CSV}")

    await scrape_range(1, 10)

    if OUTPUT_CSV.exists():
        print()
        print("=" * 60)
        print("RESULTS:")
        print("=" * 60)
        with OUTPUT_CSV.open("r", encoding="utf-8") as f:
            lines = f.readlines()
            print(f"CSV row count: {len(lines)}")
            print()
            print("First 10 lines:")
            for i, line in enumerate(lines[:10], 1):
                print(f"{i}: {line.strip()}")
    else:
        print("ERROR: CSV file was not created!")
        return False

    return True


if __name__ == "__main__":
    try:
        result = asyncio.run(test_scraper())
        if result:
            print()
            print("✓ Test passed.")
            sys.exit(0)
        else:
            print()
            print("✗ Test failed.")
            sys.exit(1)
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n\nERROR: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

