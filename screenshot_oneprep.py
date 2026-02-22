import asyncio
from playwright.async_api import async_playwright

async def snap():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        await page.wait_for_timeout(3000)
        await page.screenshot(path="oneprep_shot.png", full_page=True)
        await browser.close()

if __name__ == "__main__":
    asyncio.run(snap())
