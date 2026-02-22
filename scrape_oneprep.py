import asyncio
from playwright.async_api import async_playwright
import sqlite3
import re
import json

DB_PATH = "/home/regulus/.local/share/sat-stream/sat-stream.db"

async def setup_db():
    conn = sqlite3.connect(DB_PATH)
    return conn

async def scrape_questions():
    conn = await setup_db()
    
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()

        print("Starting scrape sequence...")
        # Start with a specific question module
        url = "https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=42"
        await page.goto(url, wait_until="networkidle")
        await page.wait_for_timeout(3000)
        
        # Now we are on a question page. Let's dump the text content of the body.
        body_text = await page.evaluate("document.body.innerText")
        
        print("Page text dump for logic building:")
        print("="*40)
        print(body_text)
        print("="*40)
        
        await browser.close()

if __name__ == "__main__":
    asyncio.run(scrape_questions())
