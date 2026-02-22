import asyncio
from playwright.async_api import async_playwright
from bs4 import BeautifulSoup

async def extract_links():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()
        print("Loading...")
        await page.goto("https://www.oneprep.xyz/question-bank", wait_until="networkidle")
        await page.wait_for_timeout(2000)
        
        html = await page.content()
        soup = BeautifulSoup(html, "html.parser")
        
        print("\nAll Links:")
        links = set()
        for a in soup.find_all('a', href=True):
            links.add(a['href'])
            
        for l in sorted(links):
            print(l)
            
        print("\nPossible buttons/cards:")
        # Look for elements that might act as links (like divs with specific classes)
        # Next.js often uses <Link href="..."> which renders as <a>, so 'a' tags should be there.
        # But if they use router.push on click, we just look at the text of clickable things.

        await browser.close()

if __name__ == "__main__":
    asyncio.run(extract_links())
