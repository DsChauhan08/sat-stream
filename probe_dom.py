import asyncio
from playwright.async_api import async_playwright
import json

async def scrape_page():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context()
        page = await context.new_page()

        print("Navigating...")
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        
        # Wait for the question elements to appear
        # The site might have a list or a grid. We need to inspect the DOM.
        print("Dumping body text to analyze structure...")
        
        # Give it a second to render
        await page.wait_for_timeout(2000)
        
        # Get all text from the body to see what's rendered
        body_text = await page.evaluate("document.body.innerText")
        print(body_text[:1000])
        
        # Also grab raw HTML to see classes
        html = await page.content()
        with open("page_dump.html", "w") as f:
            f.write(html)
            
        await browser.close()

if __name__ == "__main__":
    asyncio.run(scrape_page())
