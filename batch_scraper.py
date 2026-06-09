import asyncio
import csv
import logging
from pathlib import Path
from tqdm import tqdm
from lib.justice_fetcher import fetch_justice_financials

INPUT_CSV = Path("PO_ALL.csv")
OUTPUT_CSV = Path("financials_links.csv")
MAX_CONCURRENT = 10

logging.basicConfig(
    level=logging.WARNING,
    format="%(asctime)s %(levelname)8s | %(message)s",
    handlers=[logging.FileHandler("batch_scraper.log", "a", "utf-8")],
)

async def worker(queue: asyncio.Queue, progress, lock, writer, f):
    while True:
        ico = await queue.get()
        try:
            res = await fetch_justice_financials(ico)
            
            rows_to_write = []
            if res["financials"]:
                for fin in res["financials"]:
                    rows_to_write.append([
                        res["ico"],
                        res["subjektId"],
                        fin.get("dokumentId", ""),
                        fin["year"],
                        fin["documentUrl"],
                        fin["sourceType"],
                        fin["parserUsed"],
                        "|".join(fin["sourceNotes"])
                    ])
            else:
                rows_to_write.append([
                    res["ico"],
                    res.get("subjektId", ""),
                    "",
                    "",
                    "",
                    "",
                    "",
                    "no_records"
                ])
                
            async with lock:
                for row in rows_to_write:
                    writer.writerow(row)
                f.flush()
                    
        except Exception as e:
            logging.error(f"Error processing ICO {ico}: {e}")
        finally:
            progress.update(1)
            queue.task_done()

async def main():
    processed_icos = set()
    if OUTPUT_CSV.exists():
        with OUTPUT_CSV.open("r", encoding="utf-8") as f:
            reader = csv.reader(f)
            try:
                next(reader, None)
                for row in reader:
                    if row:
                        processed_icos.add(row[0])
            except Exception:
                pass
                
    icos = []
    with INPUT_CSV.open("r", encoding="utf-8") as f:
        reader = csv.reader(f)
        next(reader, None)  # skip header
        for row in reader:
            if row:
                ico = row[0].strip()
                if ico not in processed_icos:
                    icos.append(ico)
                    
    print(f"Total entries found. Already processed: {len(processed_icos)}. Remaining: {len(icos)}")
    if not icos:
        return

    mode = "a" if OUTPUT_CSV.exists() else "w"
    
    queue = asyncio.Queue()
    for ico in icos:
        queue.put_nowait(ico)
        
    f = open(OUTPUT_CSV, mode, newline="", encoding="utf-8")
    try:
        writer = csv.writer(f)
        if mode == "w":
            writer.writerow(["ico", "subjektId", "dokumentId", "year", "documentUrl", "sourceType", "parserUsed", "sourceNotes"])
            f.flush()
            
        lock = asyncio.Lock()
        progress = tqdm(total=len(icos), desc="Scraping ICOs")
        
        workers = [asyncio.create_task(worker(queue, progress, lock, writer, f)) for _ in range(MAX_CONCURRENT)]
        
        await queue.join()
        
        for w in workers:
            w.cancel()
    finally:
        f.close()

if __name__ == "__main__":
    asyncio.run(main())
