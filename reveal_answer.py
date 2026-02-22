import asyncio
from playwright.async_api import async_playwright

async def reveal_answer():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()

        url = "https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=42"
        await page.goto(url, wait_until="networkidle")
        await page.wait_for_timeout(3000)
        
        # Click the Explanation button
        print("Clicking 'Explanation'...")
        try:
            await page.get_by_role("button", name="Explanation").click()
            await page.wait_for_timeout(3000)
        except Exception as e:
            print(f"Could not click Explanation: {e}")
            
        # Also try to click 'Submit' if it's there
        # but just Explanation might be enough
        html = await page.content()
        with open("answer_dump.html", "w") as f:
            f.write(html)
            
        # Extract text clearly
        text = await page.evaluate("document.body.innerText")
        with open("answer_text.txt", "w") as f:
            f.write(text)
            
        await browser.close()

if __name__ == "__main__":
    asyncio.run(reveal_answer())
