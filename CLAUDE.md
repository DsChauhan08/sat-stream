# Project: SAT-Stream (Linux TUI)

## 1. Project Vision
Build a high-performance, aesthetically pleasing Terminal User Interface (TUI) application for Linux called `sat-stream`. The goal is to provide a continuous, infinite stream of SAT Math and English questions sourced from official College Board question banks and major prep books (Black Book, Princeton Review, Barron's, Erica Meltzer, College Panda).

The application must function as a "download once, run always" binary. It requires intelligent domain tracking, error analysis, and integration with the Gemini API for personalized hints and question generation based on user weaknesses.

## 2. Technology Stack
**Language:** Rust
**Why Rust?**
- Compiles to a single static binary (no runtime dependencies like Python's venv/pip).
- Performance is instant; UI animations are smooth (60fps in terminal).
- Memory safety ensures long study sessions don't leak memory.
- Best-in-class TUI library: `ratatui` (the standard for modern Rust TUIs).

**Key Crates (Dependencies):**
- `ratatui`: For the elegant "opencode-style" TUI interface.
- `crossterm`: For terminal event handling.
- `serde` / `serde_json`: For data serialization (questions/progress).
- `sqlx` (with SQLite feature): To store the massive question bank and progress locally.
- `reqwest`: For calling Gemini API.
- `tokio`: For async runtime (smooth UI while fetching AI data).

## 3. Domain Architecture
The SAT is currently Digital. We must structure the database around the official BlueBook domains but expand them for granular tracking.

### English (Verbal)
1.  **Information and Ideas**: Central Ideas, Details, Command of Evidence (Textual/Quantitative).
2.  **Craft and Structure**: Inferences, Words in Context, Text Structure/Purpose.
3.  **Expression of Ideas**: Rhetorical Synthesis, Transitions.
4.  **Standard English Conventions**: Boundaries (punctuation/clauses), Form/Structure/ Sense (grammar rules).
    - *Sub-domains*: Subject-Verb Agreement, Modifiers, Possessives, Punctuation logic.

### Mathematics
1.  **Algebra**: Linear equations, Inequalities, Systems.
2.  **Advanced Math**: Non-linear functions, Polynomials, Exponents.
3.  **Problem Solving & Data Analysis**: Ratios, Rates, Proportions, Statistics, Probability.
4.  **Geometry & Trigonometry**: Area, Volume, Angles, Circles, Trig ratios.

## 4. Data Strategy & Scraping
**Official Sources:**
- College Board Question Bank (https://satsuiteeducatorquestionbank.collegeboard.org/)
- BlueBook App Extraction (if accessible via local files)

**Prep Book Integration (PDF Parsing):**
- **Tools**: Use `pdf-extract` or `lopdf` crates in Rust to parse text.
- **Manual/AI Hybrid**: Since PDFs (Princeton, Barron's, Meltzer) are unstructured, the agent must write a parser script to extract questions.
    - *Strategy*: Look for patterns like "Question 1...", "A)", "B)".
    - *LaTeX/Math*: Extract math expressions. Convert common formats to readable text or Unicode for the TUI.

**Database Schema (SQLite):**
- `questions`: id, text, options (JSON), correct_answer, domain, sub_domain, source (e.g., "College Board", "Princeton"), difficulty.
- `user_progress`: id, question_id, is_correct, timestamp, attempts.
- `error_analysis`: id, question_id, error_type (conceptual vs. careless), notes.

## 5. Features Implementation

### A. The "Infinite Stream" Mode
- Logic: Pull questions randomly from the database.
- Filters: Allow user to toggle "Focus Mode" (e.g., only Algebra, or "Weakness Focus").
- State Persistence: On exit (`Ctrl+C` or `q`), save the current question ID and queue to a `state.json` file. On restart, resume exactly there.

### B. Domain & Sub-Domain Tracking
- Dynamic Sub-domains: If a user gets a question wrong under "Standard English Conventions", the system should try to tag the specific sub-domain (e.g., "Punctuation: Comma Splice") based on keywords in the question text or using a lightweight local classifier.
- If the sub-domain doesn't exist, create a new tag in the database.

### C. The "Error Matrix" & AI Integration
- **The Screen**: A beautiful dashboard (like GitHub's contribution graph) showing correct/incorrect patterns.
- **Incorrectness Rating**:
    - Rating = `(Times Wrong) / (Times Attempted) * Difficulty_Weight`.
    - Sort the "Review List" by this rating.
- **Gemini API Integration**:
    - **Prompt Engineering**: "I got this question wrong: [Question]. I struggle with [Domain]. Explain the concept and generate 3 similar practice questions."
    - **Config**: Store API key in `.env` file or config file in `~/.config/sat-stream/`.

## 6. UI/UX Design (The "OpenCode" Standard)
The UI must be clean, minimal, and use specific colors (Blue/Yellow/White usually).

**Layout:**
```
┌──────────────────────────────────────────────────────────────┐
│ SAT-Stream | Domain: Algebra II | Score: 89% | Streak: 5     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   Question #402 (Source: College Panda)                      │
│                                                              │
│   If f(x) = x^2 - 3x and g(x) = 2x + 1, what is f(g(2))?     │
│                                                              │
│   A) 3                                                       │
│   B) 15                                                      │
│   C) -5                                                      │
│   D) 7                                                       │
│                                                              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ [Enter] Submit | [S] Skip | [H] Hint (AI) | [Q] Quit         │
└──────────────────────────────────────────────────────────────┘
```

**Visual Feedback:**
- Correct Answer: Flash Green border, "Correct!" popup.
- Wrong Answer: Flash Red border, show correct answer, option to view AI explanation.

## 7. Agent Instructions (How to Build)
1.  **Setup**: Initialize a Rust project (`cargo init`). Add dependencies to `Cargo.toml`.
2.  **Database**: Create a migration script to set up SQLite.
3.  **Scraping**: Write a separate binary (in `src/bin/scrape.rs`) to parse the provided PDFs and URLs. Store results in DB.
    - *Note*: Web scraping of some sites may violate ToS. For the official agent execution, prioritize parsing the local PDF files provided (Princeton, Barron's, Meltzer, College Panda) as they are the most high-value dense sources. For College Board, prefer manual CSV export if available, or use the provided links responsibly.
4.  **TUI Loop**: Implement the main event loop using `ratatui`. Handle keyboard inputs.
5.  **AI Client**: Create a module for Gemini API calls. Handle errors gracefully (e.g., no internet).
6.  **Compilation**: Build with `cargo build --release`. The binary will be in `target/release/sat-stream`.

## 8. Skills & MCPs (Model Context Protocol)
- **MCP Usage**: If available, use the `filesystem` MCP to read the PDF files efficiently.
- **Search Skill**: Use search tools to verify current SAT domain names if unsure.
- **Coding Skill**: Write robust Rust code. Handle `Result<T, E>` types properly to prevent crashes.

## 9. Resources & Input Files
The user has provided the following PDF resources. Parse these first:
- `Collage Panda maths book pdf`
- `Princeton Review Digital SAT Prep Premium.pdf`
- `Barrons Digital SAT Study Guide Premium 2024.pdf`
- `Erica L. Meltzer - The Ultimate Guide to SAT Grammar.pdf`

**Goal**: Create a robust, offline-first study tool that feels like a premium application, not a script.
