import asyncio
from playwright.async_api import async_playwright

async def find_links():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        
        links = await page.evaluate(r"""
            Array.from(document.querySelectorAll('a')).map(a => a.href).filter(href => href.includes('question-bank') && href !== 'https://www.oneprep.xyz/question-bank')
        """)
        
        print("Found Topic Links:")
        for link in set(links):
            print(link)
            
        await browser.close()

if __name__ == "__main__":
    asyncio.run(find_links())
