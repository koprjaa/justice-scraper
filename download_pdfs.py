import asyncio
import csv
import logging
import re
from pathlib import Path
import html
import aiohttp
from urllib.parse import urljoin
from tqdm import tqdm
import io
import pdfplumber

INPUT_CSV = Path("financials_links.csv")
DOWNLOAD_DIR = Path("texts")
MAX_CONCURRENT = 10

logging.basicConfig(
    level=logging.WARNING,
    format="%(asctime)s %(levelname)8s | %(message)s",
    handlers=[logging.FileHandler("pdf_downloader.log", "a", "utf-8")],
)

async def download_worker(queue: asyncio.Queue, progress: tqdm):
    headers = {
        "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    }
    
    timeout = aiohttp.ClientTimeout(total=30)
    
    async with aiohttp.ClientSession(timeout=timeout, headers=headers) as session:
        while True:
            ico, subjekt_id, dokument_id, year = await queue.get()
            try:
                # Target filename
                out_path = DOWNLOAD_DIR / f"{ico}_{year}_{dokument_id}.txt"
                if out_path.exists():
                    continue

                # 1. Load the listing page to start a valid session and get the detail link
                list_url = f"https://or.justice.cz/ias/ui/vypis-sl-firma?subjektId={subjekt_id}"
                async with session.get(list_url) as resp:
                    if resp.status != 200:
                        logging.error(f"{ico}: Failed to load listing page (HTTP {resp.status})")
                        continue
                    html_content = await resp.text()

                # Find the detail link with the session token (&spis=...)
                links = re.findall(rf'href="([^"]*dokument={dokument_id}[^"]*)"', html_content)
                if not links:
                    logging.error(f"{ico}: Dokument {dokument_id} not found on listing page.")
                    continue
                
                detail_url = urljoin(list_url, html.unescape(links[0]))

                # 2. Load the detail page using the valid session link
                async with session.get(detail_url, headers={"Referer": list_url}) as resp:
                    if resp.status != 200:
                        logging.error(f"{ico}: Failed to load detail page (HTTP {resp.status})")
                        continue
                    detail_html = await resp.text()

                if "Neplatný odkaz" in detail_html:
                    logging.error(f"{ico}: Detail page returned 'Neplatný odkaz' despite session flow.")
                    continue

                # Find the actual download link (the one that expires)
                dl_links = re.findall(r'href="([^"]+)"', detail_html)
                dl_matches = [l for l in dl_links if 'download' in l and 'GDPR' not in l]
                
                if not dl_matches:
                    logging.error(f"{ico}: No download link found on detail page.")
                    continue
                    
                download_url = urljoin(detail_url, dl_matches[0])

                # 3. Download the actual file
                async with session.get(download_url, headers={"Referer": detail_url}) as resp:
                    if resp.status == 200:
                        content_type = resp.headers.get("Content-Type", "")
                        if "text/html" in content_type:
                            logging.error(f"{ico}: Download returned HTML instead of a file.")
                            continue
                            
                        file_data = await resp.read()
                        text_chunks = []
                        try:
                            with pdfplumber.open(io.BytesIO(file_data)) as pdf:
                                for page in pdf.pages:
                                    page_text = page.extract_text() or ""
                                    if page_text.strip():
                                        text_chunks.append(page_text)
                            extracted_text = "\n\n".join(text_chunks)
                            with out_path.open("w", encoding="utf-8") as f:
                                f.write(extracted_text)
                        except Exception as e:
                            logging.error(f"{ico}: Failed to extract text from PDF: {e}")
                    else:
                        logging.error(f"{ico}: Failed to download file (HTTP {resp.status})")

            except Exception as e:
                logging.error(f"{ico}: Exception during download: {e}")
            finally:
                progress.update(1)
                queue.task_done()

async def main():
    DOWNLOAD_DIR.mkdir(exist_ok=True)
    
    if not INPUT_CSV.exists():
        print(f"File {INPUT_CSV} not found. Run batch_scraper.py first.")
        return

    tasks = []
    with INPUT_CSV.open("r", encoding="utf-8") as f:
        reader = csv.reader(f)
        header = next(reader, None)
        if not header: return
        
        # Expecting: ico, subjektId, dokumentId, year, documentUrl, sourceType, parserUsed, sourceNotes
        for row in reader:
            if len(row) >= 4:
                ico, subjekt_id, dokument_id, year = row[0], row[1], row[2], row[3]
                if dokument_id and subjekt_id:  # Only if we found a document
                    tasks.append((ico, subjekt_id, dokument_id, year))

    print(f"Found {len(tasks)} documents to download.")
    if not tasks:
        return

    queue = asyncio.Queue()
    for t in tasks:
        queue.put_nowait(t)

    progress = tqdm(total=len(tasks), desc="Extracting text from PDFs")
    workers = [asyncio.create_task(download_worker(queue, progress)) for _ in range(MAX_CONCURRENT)]

    await queue.join()

    for w in workers:
        w.cancel()

if __name__ == "__main__":
    asyncio.run(main())
