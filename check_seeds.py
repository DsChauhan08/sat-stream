import asyncio
from playwright.async_api import async_playwright

async def check_seeds():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page()

        print("Testing seed=42...")
        await page.goto("https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=42", wait_until="networkidle")
        await page.wait_for_timeout(2000)
        q1 = await page.evaluate("document.querySelector('.question-stem') ? document.querySelector('.question-stem').innerText : 'None'")
        print(f"Q1: {q1[:50]}...")

        print("Testing seed=43...")
        await page.goto("https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=43", wait_until="networkidle")
        await page.wait_for_timeout(2000)
        q2 = await page.evaluate("document.querySelector('.question-stem') ? document.querySelector('.question-stem').innerText : 'None'")
        print(f"Q2: {q2[:50]}...")

        print("Testing seed=44...")
        await page.goto("https://www.oneprep.xyz/api/questions/first?question_set=unified&module=math&seed=44", wait_until="networkidle")
        await page.wait_for_timeout(2000)
        q3 = await page.evaluate("document.querySelector('.question-stem') ? document.querySelector('.question-stem').innerText : 'None'")
        print(f"Q3: {q3[:50]}...")

        await browser.close()

if __name__ == "__main__":
    asyncio.run(check_seeds())
