#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path


DOMAIN_RW = {
    1: "Craft and Structure",
    2: "Craft and Structure",
    3: "Craft and Structure",
    4: "Craft and Structure",
    5: "Craft and Structure",
    6: "Information and Ideas",
    7: "Information and Ideas",
    8: "Information and Ideas",
    9: "Information and Ideas",
    10: "Information and Ideas",
    11: "Information and Ideas",
    12: "Information and Ideas",
    13: "Information and Ideas",
    14: "Information and Ideas",
    15: "Standard English Conventions",
    16: "Standard English Conventions",
    17: "Standard English Conventions",
    18: "Standard English Conventions",
    19: "Standard English Conventions",
    20: "Standard English Conventions",
    21: "Expression of Ideas",
    22: "Expression of Ideas",
    23: "Expression of Ideas",
    24: "Expression of Ideas",
    25: "Expression of Ideas",
    26: "Expression of Ideas",
    27: "Expression of Ideas",
}

DOMAIN_MATH = {
    1: "Algebra",
    2: "Algebra",
    3: "Algebra",
    4: "Geometry & Trigonometry",
    5: "Algebra",
    6: "Problem Solving & Data Analysis",
    7: "Problem Solving & Data Analysis",
    8: "Advanced Math",
    9: "Advanced Math",
    10: "Advanced Math",
    11: "Geometry & Trigonometry",
    12: "Algebra",
    13: "Advanced Math",
    14: "Algebra",
    15: "Problem Solving & Data Analysis",
    16: "Advanced Math",
    17: "Geometry & Trigonometry",
    18: "Algebra",
    19: "Geometry & Trigonometry",
    20: "Advanced Math",
    21: "Advanced Math",
    22: "Advanced Math",
}


def clean(s: str) -> str:
    s = s.replace("\u000c", "\n")
    s = re.sub(r"\s+", " ", s)
    return s.strip()


def parse_princeton_answer_key(lines: list[str]) -> dict[str, dict[int, str]]:
    key = {"rw1": {}, "rw2": {}, "m1": {}, "m2": {}}
    mode = None
    for line in lines:
        t = line.strip()
        if "Reading and Writing Comprehension—Module 1" in t:
            mode = "rw1"
            continue
        if "Reading and Writing Comprehension—Module 2: Easier" in t:
            mode = "rw2"
            continue
        if t == "Math—Module 1":
            mode = "m1"
            continue
        if t == "Math—Module 2: Easier":
            mode = "m2"
            continue
        if mode is None:
            continue

        m = re.match(r"^\s*(\d{1,2})\s+([A-D])\b", t)
        if m:
            key[mode][int(m.group(1))] = m.group(2)
    return key


def extract_princeton_stem_options(block_lines: list[str]) -> tuple[str, list[str]]:
    lines = [ln.rstrip() for ln in block_lines]

    # Remove leading prompt boilerplate line if present.
    while lines and "Mark for Review" in lines[0]:
        lines.pop(0)

    # Trim at clear section boundary markers if they exist in the block.
    trimmed = []
    for ln in lines:
        s = ln.strip()
        if s.startswith("Section 1, Module") or s.startswith("Section 2, Module"):
            break
        if s.startswith("SAT Prep Test") or s.startswith("DIRECTIONS"):
            break
        if s == "Text 2":
            # keep Text 2 only as part of prompt; for options stop before this marker
            break
        trimmed.append(ln)
    lines = trimmed

    # Stem runs until first A/B/C/D option header.
    i = 0
    stem_lines = []
    while i < len(lines):
        if re.match(r"^\s*[A-D]\s+", lines[i]):
            break
        stem_lines.append(lines[i])
        i += 1

    options = []
    current = ""
    while i < len(lines):
        m = re.match(r"^\s*([A-D])\s+(.*)$", lines[i])
        if m:
            if current:
                options.append(clean(current))
            current = m.group(2).strip()
        else:
            if current:
                if re.match(r"^\s*\d{1,2}\.\s+Mark for Review\s*$", lines[i]):
                    break
                s = lines[i].strip()
                if s.startswith("Section 1, Module") or s.startswith(
                    "Section 2, Module"
                ):
                    break
                if s.startswith("SAT Prep Test") or s.startswith("DIRECTIONS"):
                    break
                current += " " + lines[i].strip()
        i += 1
    if current:
        options.append(clean(current))

    stem = clean(" ".join(stem_lines))
    cut = stem.find("Which choice")
    if cut != -1:
        stem = clean(stem[:cut])
    return stem, options[:4]


def parse_princeton(princeton_txt: Path) -> list[dict]:
    text = princeton_txt.read_text(encoding="utf-8", errors="ignore")
    lines = text.splitlines()

    starts = []
    for i, line in enumerate(lines):
        m = re.match(r"^\s*(\d{1,2})\.\s+Mark for Review\s*$", line)
        if m:
            starts.append((i, int(m.group(1))))

    if len(starts) < 98:
        return []
    starts = starts[:98]  # SAT Prep Test 1 only

    key = parse_princeton_answer_key(lines)

    questions = []
    for idx, (line_no, qnum) in enumerate(starts):
        next_line = starts[idx + 1][0] if idx + 1 < len(starts) else len(lines)
        block = lines[line_no + 1 : next_line]
        stem, opts = extract_princeton_stem_options(block)
        if len(opts) != 4 or not stem:
            continue

        if idx < 27:
            section = "english"
            domain = DOMAIN_RW.get(qnum, "Information and Ideas")
            ans = key["rw1"].get(qnum)
            source = "Princeton Test1 RW Module1"
        elif idx < 54:
            section = "english"
            domain = DOMAIN_RW.get(qnum, "Information and Ideas")
            ans = key["rw2"].get(qnum)
            source = "Princeton Test1 RW Module2"
        elif idx < 76:
            section = "math"
            domain = DOMAIN_MATH.get(qnum, "Algebra")
            ans = key["m1"].get(qnum)
            source = "Princeton Test1 Math Module1"
        else:
            section = "math"
            domain = DOMAIN_MATH.get(qnum, "Algebra")
            ans = key["m2"].get(qnum)
            source = "Princeton Test1 Math Module2"

        if ans not in {"A", "B", "C", "D"}:
            continue

        questions.append(
            {
                "source": source,
                "section": section,
                "domain": domain,
                "sub_domain": "",
                "difficulty": 2,
                "passage": "",
                "question_text": stem,
                "options": opts,
                "correct_answer": ans,
                "explanation": "",
                "media_json": "[]",
            }
        )

    return questions


def find_nth(text: str, needle: str, n: int) -> int:
    pos = -1
    start = 0
    for _ in range(n):
        pos = text.find(needle, start)
        if pos == -1:
            return -1
        start = pos + 1
    return pos


def parse_barrons_key_flat(block: str) -> dict[int, str]:
    out = {}
    for q, a in re.findall(r"\b(\d{1,2})\.\s*([A-D])\b", block):
        qq = int(q)
        if 1 <= qq <= 27:
            out[qq] = a
    return out


def parse_barrons_answer_keys(text: str) -> dict[str, dict[int, str]]:
    keys = {
        "diag_rw1": {},
        "diag_rw2": {},
        "diag_m1": {},
        "diag_m2": {},
        "p1_rw1": {},
        "p1_rw2": {},
        "p1_m1": {},
        "p1_m2": {},
    }

    # Diagnostic split keys.
    d1 = text.find("ANSWER KEY\n                               Diagnostic Test")
    d2 = text.find("ANSWER KEY\n                        Diagnostic Test")
    if d1 != -1:
        block = text[d1 : d1 + 20000]
        rw1_pos = block.find("Reading and Writing Module 1")
        rw2_pos = block.find("Reading and Writing Module 2")
        if rw1_pos != -1 and rw2_pos != -1:
            keys["diag_rw1"] = parse_barrons_key_flat(block[rw1_pos:rw2_pos])
            keys["diag_rw2"] = parse_barrons_key_flat(block[rw2_pos:])
    if d2 != -1:
        block = text[d2 : d2 + 15000]
        m1_pos = block.find("Math Module 1")
        m2_pos = block.find("Math Module 2")
        if m1_pos != -1 and m2_pos != -1:
            keys["diag_m1"] = parse_barrons_key_flat(block[m1_pos:m2_pos])
            keys["diag_m2"] = parse_barrons_key_flat(block[m2_pos:])

    # Practice test 1 key.
    p1 = text.find("ANSWER KEY\n                               Practice Test 1")
    if p1 != -1:
        block = text[p1 : p1 + 25000]
        s_rw1 = block.find("Section 1, Module 1: Reading and Writing")
        s_rw2 = block.find("Section 1, Module 2: Reading and Writing")
        s_m1 = block.find("Section 2, Module 1: Math")
        s_m2 = block.find("Section 2, Module 2: Math")
        if min(s_rw1, s_rw2, s_m1, s_m2) != -1:
            keys["p1_rw1"] = parse_barrons_key_flat(block[s_rw1:s_rw2])
            keys["p1_rw2"] = parse_barrons_key_flat(block[s_rw2:s_m1])
            keys["p1_m1"] = parse_barrons_key_flat(block[s_m1:s_m2])
            keys["p1_m2"] = parse_barrons_key_flat(block[s_m2:])

    return keys


def split_question_blocks(lines: list[str], max_q: int) -> list[tuple[int, list[str]]]:
    starts = []
    for i, line in enumerate(lines):
        m = re.match(r"^\s*(\d{1,2})\s*$", line)
        if m:
            q = int(m.group(1))
            if 1 <= q <= max_q:
                starts.append((i, q))

    blocks = []
    for idx, (line_no, qnum) in enumerate(starts):
        nxt = starts[idx + 1][0] if idx + 1 < len(starts) else len(lines)
        blocks.append((qnum, lines[line_no + 1 : nxt]))
    return blocks


def extract_barrons_stem_options(block_lines: list[str]) -> tuple[str, list[str]]:
    lines = [ln.rstrip() for ln in block_lines]

    # Find first line with a question mark as the end of the prompt sentence.
    qidx = -1
    for i, ln in enumerate(lines):
        if "?" in ln:
            qidx = i
            break
    if qidx == -1:
        return "", []

    stem = clean(" ".join(x.strip() for x in lines[: qidx + 1]))
    # Trim helper sentence if present.
    cut = stem.find("Which choice")
    if cut != -1:
        stem = clean(stem[:cut])

    opts = []
    for ln in lines[qidx + 1 :]:
        s = ln.strip()
        if not s:
            continue
        if re.fullmatch(r"\d{1,2}", s):
            break
        if s.startswith("Section ") or s.startswith("ANSWER KEY"):
            break
        if s.startswith("Text 1") or s.startswith("Text 2"):
            break
        if "The following text" in s or "The student wants" in s:
            break
        if s.startswith("In her essay") or s.startswith("Psychology Today"):
            break
        parts = [p.strip() for p in re.split(r"\s{2,}", s) if p.strip()]
        if not parts:
            continue
        for part in parts:
            if part in {"A", "B", "C", "D"}:
                continue
            opts.append(clean(part))
            if len(opts) == 4:
                merged = merge_split_options(opts)
                return stem, merged
    return stem, merge_split_options(opts[:4])


def merge_split_options(opts: list[str]) -> list[str]:
    if len(opts) != 4:
        return opts
    merged = opts[:]
    for i in range(1, 4):
        if len(merged[i].split()) <= 2 and len(merged[i - 1].split()) >= 2:
            merged[i - 1] = clean(merged[i - 1] + " " + merged[i])
            merged[i] = ""
    merged = [o for o in merged if o]
    return merged[:4]


def build_barrons_section_questions(
    section_text: str,
    section: str,
    domain_map: dict[int, str],
    key_map: dict[int, str],
    source: str,
) -> list[dict]:
    if not section_text:
        return []
    lines = section_text.replace("\u000c", "\n").splitlines()
    max_q = 27 if section == "english" else 22
    blocks = split_question_blocks(lines, max_q)

    out = []
    for qnum, block in blocks:
        stem, opts = extract_barrons_stem_options(block)
        if len(opts) != 4 or not stem:
            continue
        ans = key_map.get(qnum)
        if ans not in {"A", "B", "C", "D"}:
            continue
        out.append(
            {
                "source": source,
                "section": section,
                "domain": domain_map.get(
                    qnum, "Information and Ideas" if section == "english" else "Algebra"
                ),
                "sub_domain": "",
                "difficulty": 2,
                "passage": "",
                "question_text": stem,
                "options": opts,
                "correct_answer": ans,
                "explanation": "",
                "media_json": "[]",
            }
        )
    return out


def parse_barrons(barrons_txt: Path) -> list[dict]:
    text = barrons_txt.read_text(encoding="utf-8", errors="ignore")
    keys = parse_barrons_answer_keys(text)

    # Relevant section occurrences from this file:
    # - Diagnostic actual test starts at occurrence #5 for RW1, #4 for RW2, #5 for M1, #4 for M2.
    # - Practice Test 1 actual starts at occurrence #7 for RW1, #5 for RW2, #7 for M1, #5 for M2.
    marker_rw1 = "Section 1, Module 1: Reading and Writing"
    marker_rw2_diag = "Section 1, Module 2: Reading and Writing"
    marker_rw2_p1 = "Section 1, Module 2, Reading and Writing"
    marker_m1 = "Section 2, Module 1: Math"
    marker_m2 = "Section 2, Module 2: Math"

    d_rw1_pos = find_nth(text, marker_rw1, 5)
    d_rw2_pos = find_nth(text, marker_rw2_diag, 4)
    d_m1_pos = find_nth(text, marker_m1, 5)
    d_m2_pos = find_nth(text, marker_m2, 4)

    p_rw1_pos = find_nth(text, marker_rw1, 7)
    p_rw2_pos = find_nth(text, marker_rw2_p1, 5)
    p_m1_pos = find_nth(text, marker_m1, 7)
    p_m2_pos = find_nth(text, marker_m2, 5)

    diag_rw1 = text[d_rw1_pos:d_rw2_pos] if d_rw1_pos != -1 and d_rw2_pos != -1 else ""
    diag_rw2 = text[d_rw2_pos:d_m1_pos] if d_rw2_pos != -1 and d_m1_pos != -1 else ""
    diag_m1 = text[d_m1_pos:d_m2_pos] if d_m1_pos != -1 and d_m2_pos != -1 else ""
    diag_m2 = (
        text[d_m2_pos : text.find("ANSWER KEY", d_m2_pos)] if d_m2_pos != -1 else ""
    )

    p1_rw1 = text[p_rw1_pos:p_rw2_pos] if p_rw1_pos != -1 and p_rw2_pos != -1 else ""
    p1_rw2 = text[p_rw2_pos:p_m1_pos] if p_rw2_pos != -1 and p_m1_pos != -1 else ""
    p1_m1 = text[p_m1_pos:p_m2_pos] if p_m1_pos != -1 and p_m2_pos != -1 else ""
    p1_m2 = text[p_m2_pos : text.find("ANSWER KEY", p_m2_pos)] if p_m2_pos != -1 else ""

    out = []
    out.extend(
        build_barrons_section_questions(
            diag_rw1,
            "english",
            DOMAIN_RW,
            keys["diag_rw1"],
            "Barrons Diagnostic RW Module1",
        )
    )
    out.extend(
        build_barrons_section_questions(
            diag_rw2,
            "english",
            DOMAIN_RW,
            keys["diag_rw2"],
            "Barrons Diagnostic RW Module2",
        )
    )
    out.extend(
        build_barrons_section_questions(
            diag_m1,
            "math",
            DOMAIN_MATH,
            keys["diag_m1"],
            "Barrons Diagnostic Math Module1",
        )
    )
    out.extend(
        build_barrons_section_questions(
            diag_m2,
            "math",
            DOMAIN_MATH,
            keys["diag_m2"],
            "Barrons Diagnostic Math Module2",
        )
    )
    out.extend(
        build_barrons_section_questions(
            p1_rw1,
            "english",
            DOMAIN_RW,
            keys["p1_rw1"],
            "Barrons Practice1 RW Module1",
        )
    )
    out.extend(
        build_barrons_section_questions(
            p1_rw2,
            "english",
            DOMAIN_RW,
            keys["p1_rw2"],
            "Barrons Practice1 RW Module2",
        )
    )
    out.extend(
        build_barrons_section_questions(
            p1_m1,
            "math",
            DOMAIN_MATH,
            keys["p1_m1"],
            "Barrons Practice1 Math Module1",
        )
    )
    out.extend(
        build_barrons_section_questions(
            p1_m2,
            "math",
            DOMAIN_MATH,
            keys["p1_m2"],
            "Barrons Practice1 Math Module2",
        )
    )

    return out


def dedupe(items: list[dict]) -> list[dict]:
    seen = set()
    out = []
    for q in items:
        key = (q["section"], clean(q["question_text"]).lower())
        if key in seen:
            continue
        seen.add(key)
        out.append(q)
    return out


def write_questions(items: list[dict], out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for i, q in enumerate(items, start=1):
        (out_dir / f"q_{i:04d}.json").write_text(
            json.dumps(q, ensure_ascii=False, indent=2), encoding="utf-8"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--princeton", required=True)
    parser.add_argument("--barrons", required=True)
    parser.add_argument("--out", default="data/import_questions")
    args = parser.parse_args()

    p = parse_princeton(Path(args.princeton))
    b = parse_barrons(Path(args.barrons))
    all_q = dedupe(p + b)
    write_questions(all_q, Path(args.out))

    print(f"princeton: {len(p)}")
    print(f"barrons: {len(b)}")
    print(f"total_written: {len(all_q)}")


if __name__ == "__main__":
    main()
