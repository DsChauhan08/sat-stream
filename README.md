# SAT-Stream 🎓

> Your infinite SAT prep companion — a premium terminal experience powered by AI.

![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux-green)

## ✨ Features

- **🚀 Infinite Question Stream** — 200+ built-in SAT questions across all 8 official domains
- **📊 Analytics Dashboard** — GitHub-style activity heatmap, per-domain progress bars, streak tracking  
- **🤖 AI-Powered Learning** — Gemini API integration for hints, explanations, and question generation
- **⏱️ Timed Practice Mode** — Simulate real SAT timing constraints (95s/math, 71s/English)
- **🔄 Spaced Repetition** — SM-2 algorithm for optimal review scheduling
- **🎨 4 Color Themes** — Catppuccin, Tokyo Night, Dracula, Gruvbox
- **💾 Progress Persistence** — SQLite database with session tracking and daily activity logs
- **📦 Linux Packages** — Build as `.deb` or `.rpm` for easy installation

## 🚀 Quick Start

```bash
# Clone and build
git clone https://github.com/yourusername/sat-stream.git
cd sat-stream
cargo build --release

# Run
./target/release/sat-stream
```

## 📦 Installation (Linux Packages)

### Debian/Ubuntu (.deb)
```bash
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/sat-stream_*.deb
```

### Fedora/RHEL (.rpm)
```bash
cargo install cargo-generate-rpm
cargo build --release
cargo generate-rpm
sudo rpm -i target/generate-rpm/sat-stream-*.rpm
```

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate / Select answer |
| `Enter` | Confirm selection |
| `A/B/C/D` | Quick select answer |
| `G` | Open graph/figure (Kitty) |
| `H` | Get AI hint |
| `E` | Get AI explanation (after answering) |
| `M` | Change quiz mode |
| `T` | Cycle color theme |
| `1-5` | Quick navigation between screens |
| `Q/Esc` | Go back / Quit |

## 🎯 Quiz Modes

| Mode | Description |
|------|-------------|
| **Infinite Stream** | Random questions from all domains |
| **Weakness Focus** | Targets your weakest domains |
| **Spaced Review** | Reviews questions you got wrong |
| **Timed Practice** | Enforces SAT timing constraints |

## 📚 SAT Domains Covered

### Math
- Algebra (linear equations, systems, inequalities)
- Advanced Math (quadratics, polynomials, exponentials)
- Problem Solving & Data Analysis (statistics, probability, ratios)
- Geometry & Trigonometry (circles, triangles, trig functions)

### Reading & Writing
- Craft and Structure (vocabulary, text structure, tone)
- Information and Ideas (central ideas, evidence, inference)
- Standard English Conventions (grammar, punctuation, agreement)
- Expression of Ideas (transitions, conciseness, style)

## 🤖 AI Integration

Set your Gemini API key to enable AI features:

```bash
# In the app: press K in Settings
# Or set the environment variable:
export GEMINI_API_KEY="your-api-key-here"
```

Features:
- **Hints** — Get a nudge in the right direction without revealing the answer
- **Explanations** — Detailed step-by-step breakdown of correct answers
- **Question Generation** — AI generates new questions (coming soon)

## 🛠️ Building from Source

### Prerequisites
- Rust 1.75+ with `cargo`
- Linux (primary target)

```bash
cargo build --release
```

## 📁 Data Location

All data is stored in `~/.config/sat-stream/`:
- `config.toml` — Theme, API key, timing settings
- `sat_stream.db` — SQLite database with questions and progress
- `state.json` — Session resume data

## License

MIT
