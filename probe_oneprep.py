import asyncio
import json
from playwright.async_api import async_playwright

async def probe_network():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context()
        page = await context.new_page()

        async def log_response(response):
            try:
                if response.request.resource_type in ["fetch", "xhr"]:
                    url = response.url
                    # Skip tracking/analytics
                    if "googletagmanager" in url or "tally.so" in url:
                        return
                    
                    if "oneprep.xyz" in url:
                        print(f"Intercepted API Call: {url}")
                        text = await response.text()
                        print(f"Response snippet (first 200 chars): {text[:200]}\n")
            except Exception as e:
                pass

        page.on("response", log_response)

        print("Navigating to https://www.oneprep.xyz/question-bank...")
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        
        # Click the first question or interact to trigger more fetches if needed
        # We will just wait a bit for initial fetches
        await page.wait_for_timeout(5000)
        
        await browser.close()

if __name__ == "__main__":
    asyncio.run(probe_network())
