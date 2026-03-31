#!/usr/bin/env python3
"""
Export publicly reachable OnePrep question payloads into local JSON files.

This script uses Playwright to visit OnePrep question pages and extract
`initialQuestion` objects from server-rendered Next.js stream chunks when available.

Output files are written as one JSON object per question to:
  data/oneprep_export/

Usage:
  python3 scripts/export_oneprep_questions.py --module math --start 1 --end 300
  python3 scripts/export_oneprep_questions.py --module en --start 1 --end 300
"""

import argparse
import asyncio
import json
import os
import re
from pathlib import Path
from typing import Any, Dict, List, Optional

from playwright.async_api import async_playwright


OUT_DIR = Path("data/oneprep_export")


def parse_initial_question_from_html(html: str) -> Optional[Dict[str, Any]]:
    # Extract Next.js stream payload chunks embedded in script tags.
    # Pattern captures escaped string body in self.__next_f.push([1,"..."]).
    pat = re.compile(
        r'self\.__next_f\.push\(\[1,"((?:\\.|[^"\\])*)"\]\)</script>', re.S
    )
    chunks = pat.findall(html)
    for raw in chunks:
        try:
            decoded = bytes(raw, "utf-8").decode("unicode_escape")
        except Exception:
            continue
        marker = '"initialQuestion":'
        idx = decoded.find(marker)
        if idx == -1:
            continue

        tail = decoded[idx + len(marker) :]
        # Parse first JSON object by brace matching.
        if not tail or tail[0] != "{":
            continue
        brace = 0
        in_str = False
        esc = False
        end = -1
        for i, ch in enumerate(tail):
            if in_str:
                if esc:
                    esc = False
                elif ch == "\\":
                    esc = True
                elif ch == '"':
                    in_str = False
            else:
                if ch == '"':
                    in_str = True
                elif ch == "{":
                    brace += 1
                elif ch == "}":
                    brace -= 1
                    if brace == 0:
                        end = i + 1
                        break
        if end == -1:
            continue

        obj_text = tail[:end]
        try:
            obj = json.loads(obj_text)
        except Exception:
            continue
        return obj
    return None


async def fetch_question(page, module: str, seed: int) -> Optional[Dict[str, Any]]:
    url = f"https://www.oneprep.xyz/api/questions/first?question_set=unified&module={module}&seed={seed}"
    await page.goto(url, wait_until="domcontentloaded", timeout=90000)
    await page.wait_for_timeout(2000)
    html = await page.content()
    return parse_initial_question_from_html(html)


async def run(module: str, start: int, end: int) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ua = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True, args=["--disable-blink-features=AutomationControlled"]
        )
        context = await browser.new_context(user_agent=ua, locale="en-US")
        page = await context.new_page()

        ok = 0
        miss = 0

        for seed in range(start, end + 1):
            out_file = OUT_DIR / f"{module}_{seed}.json"
            if out_file.exists():
                continue

            try:
                q = await fetch_question(page, module, seed)
            except Exception as e:
                print(f"[{module}:{seed}] error: {e}")
                miss += 1
                continue

            if not q:
                print(f"[{module}:{seed}] no payload")
                miss += 1
                continue

            out_file.write_text(json.dumps(q, ensure_ascii=False, indent=2))
            ok += 1
            print(f"[{module}:{seed}] saved {q.get('id', 'unknown')}")

            if seed % 25 == 0:
                await page.wait_for_timeout(500)

        await browser.close()
        print(f"done module={module} ok={ok} miss={miss}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--module", choices=["math", "en"], required=True)
    parser.add_argument("--start", type=int, required=True)
    parser.add_argument("--end", type=int, required=True)
    args = parser.parse_args()

    asyncio.run(run(args.module, args.start, args.end))


if __name__ == "__main__":
    main()
