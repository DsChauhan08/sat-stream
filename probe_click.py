import asyncio
from playwright.async_api import async_playwright

async def interact():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()

        async def log_response(response):
            if "oneprep.xyz" in response.url and "_rsc" not in response.url and "google" not in response.url:
                try:
                    text = await response.text()
                    if "question" in text.lower() or "answer" in text.lower():
                        print(f"Intercepted Data URL: {response.url}")
                        print(f"Data snippet: {text[:500]}\n")
                except:
                    pass

        page.on("response", log_response)

        print("Navigating to question bank...")
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        
        print("Clicking a topic...")
        # Find any text that looks like a topic and click it
        topics = await page.locator("text='Linear equations in one variable'").all()
        if topics:
            await topics[0].click()
            await page.wait_for_timeout(5000)
            
        await browser.close()

if __name__ == "__main__":
    asyncio.run(interact())
