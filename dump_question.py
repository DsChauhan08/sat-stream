import asyncio
from playwright.async_api import async_playwright

async def dump_html():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()

        url = "https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=42"
        await page.goto(url, wait_until="networkidle")
        await page.wait_for_timeout(3000)
        
        html = await page.content()
        with open("question_dump.html", "w") as f:
            f.write(html)
            
        await browser.close()

if __name__ == "__main__":
    asyncio.run(dump_html())
