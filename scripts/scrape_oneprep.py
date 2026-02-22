import asyncio
import sqlite3
import os
from playwright.async_api import async_playwright

DB_PATH = "/home/regulus/.local/share/sat-stream/oneprep_raw.db"

def setup_db():
    # Store in a temporary DB before processing with AI
    os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS scraped_questions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            module TEXT,
            seed INTEGER,
            question_text TEXT,
            options_text TEXT,
            UNIQUE(module, seed)
        )
    ''')
    conn.commit()
    return conn

async def get_text_safe(page, selector):
    try:
        element = await page.query_selector(selector)
        if element:
            return await element.inner_text()
    except:
        pass
    return ""

async def scrape_all():
    conn = setup_db()
    cursor = conn.cursor()
    
    modules = ["math", "english"]
    max_seed = 2500 # Slightly above the 2161 and 2130 counts
    
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context()
        page = await context.new_page()

        # Optimize for speed: block images/fonts
        async def route_intercept(route):
            if route.request.resource_type in ["image", "font", "media"]:
                await route.abort()
            else:
                await route.continue_()
        await page.route("**/*", route_intercept)

        for module in modules:
            print(f"Starting module: {module}")
            for seed in range(1, 2600): # Full run
                # Check if we already have it
                cursor.execute("SELECT 1 FROM scraped_questions WHERE module=? AND seed=?", (module, seed))
                if cursor.fetchone():
                    continue
                    
                url = f"https://www.oneprep.xyz/api/questions/first?question_set=unified&module={module}&seed={seed}"
                
                # Tiny sleep to avoid aggressive rate limits
                if seed % 50 == 0:
                    print(f"[{module.upper()}] Reached seed {seed}, taking a short 5s breather...")
                    await asyncio.sleep(5)
                try:
                    await page.goto(url, wait_until="domcontentloaded", timeout=20000)
                    # wait a tiny bit for React to render the question stem
                    await page.wait_for_selector('.question-stem', timeout=10000)
                    
                    q_text = await get_text_safe(page, '.question-stem')
                    # Options are usually radio buttons or similar. We can grab all text after the question stem,
                    # or look for a specific container. Let's grab the whole body if specific options aren't found.
                    # Or grab text of elements with like 'A', 'B', 'C', 'D' prefixes.
                    # On the DOM dump earlier, we didn't see options for the math question.
                    # Let's extract all text from the main container just in case.
                    o_text = await get_text_safe(page, '.options-container, .answers, form')
                    
                    if q_text:
                        cursor.execute(
                            "INSERT OR IGNORE INTO scraped_questions (module, seed, question_text, options_text) VALUES (?, ?, ?, ?)",
                            (module, seed, q_text, o_text)
                        )
                        conn.commit()
                        print(f"[{module.upper()}] Saved seed {seed}: {q_text[:40]}...")
                    else:
                        print(f"[{module.upper()}] Seed {seed} has no question text.")
                        
                except Exception as e:
                    print(f"Error on {module} seed {seed}: {e}")
                    
        await browser.close()
        
if __name__ == "__main__":
    asyncio.run(scrape_all())
