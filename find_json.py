import re
import json

def find_json():
    with open("question_dump.html", "r", encoding="utf-8") as f:
        html = f.read()

    # Look for any JSON-like structures containing the question text
    matches = re.finditer(r'(\{.*?40\\pi.*?\})', html)
    for m in matches:
        print("Found possible JSON chunk:", m.group(1)[:200])

if __name__ == "__main__":
    find_json()
