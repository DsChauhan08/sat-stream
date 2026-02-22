import asyncio
from playwright.async_api import async_playwright
import json

async def test_fetch():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context()
        page = await context.new_page()
        
        print("Loading base site to get session/cookies...")
        await page.goto("https://www.oneprep.xyz/question-bank")
        await page.wait_for_timeout(2000)
        
        print("Executing fetch...")
        result = await page.evaluate(r"""
            async () => {
                const res = await fetch('/api/questions/first?question_set=unified&module=math&seed=42');
                return await res.text();
            }
        """)
        
        print("Result:")
        print(result[:1000])

        await browser.close()

if __name__ == "__main__":
    asyncio.run(test_fetch())
