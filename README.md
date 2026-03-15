# Neo

**Agentic AI development platform — multi-model, CLI-first.**

Neo orchestrates multiple specialised AI agents across different models, using [OpenRouter](https://openrouter.ai) to dynamically select the strongest model for each task. Built in Rust for speed, shipped as a single binary.

![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![License](https://img.shields.io/badge/license-MIT-green)

---

## Demo

<p align="center">
  <img src="demo.gif" alt="Neo demo — CLI commands, tool use, and code review" width="800">
</p>

> One-shot questions, file reading with tool use, and automated code review — all routed to the optimal model via OpenRouter.

---

## Features

- **Intelligent model routing** — Weighted scoring algorithm selects the optimal model per task (capability 40%, cost 25%, latency 20%, context fit 15%)
- **8 specialised agents** — Router, Planner, Coder, Reviewer, Debugger, Tester, Documenter, Oracle — each with tailored system prompts and tool access
- **7 built-in tools** — File read/edit/create, regex search, glob, shell execution, git operations — all workspace-sandboxed
- **Interactive REPL** — Streaming responses, cost footer, conversation history
- **12 CLI commands** — `ask`, `do`, `review`, `plan`, `debug`, `test`, `doc`, `config`, `threads`, `resume`, `cost`, `models`
- **Budget controls** — Per-request, per-session, and per-day spending caps
- **Layered config** — Global → workspace → local TOML files + `NEO_*` environment variable overrides
- **Thread persistence** — Every conversation saved, searchable, resumable
- **Zero-config start** — Works with just an API key

---

## Install

### From source

```bash
git clone https://github.com/matthart1983/neo.git
cd neo
cargo build --release
```

### Prerequisites

- **Rust** toolchain (1.75+): https://rustup.rs
- **OpenRouter API key**: https://openrouter.ai/keys

---

## Quick Start

```bash
# Set your API key
export OPENROUTER_API_KEY=sk-or-v1-...

# Interactive REPL
neo

# One-shot question
neo ask "explain this error" < error.log

# Execute a task
neo do "add input validation to the signup form"

# Review uncommitted changes
neo review

# Plan without executing
neo plan "migrate from REST to GraphQL"

# Debug a failing test
neo debug "test_auth fails with 401"

# Generate tests
neo test

# Update documentation
neo doc
```

---

## Commands

| Command | Description |
|---------|-------------|
| `neo` | Start interactive REPL |
| `neo ask "<prompt>"` | One-shot question |
| `neo do "<task>"` | Execute a task end-to-end |
| `neo review [commit]` | Review changes (defaults to uncommitted) |
| `neo plan "<task>"` | Produce an implementation plan |
| `neo debug "<error>"` | Diagnose an error |
| `neo test` | Generate tests for changed files |
| `neo doc` | Generate/update documentation |
| `neo config` | Show current configuration |
| `neo config set <key> <val>` | Update a config value |
| `neo threads [--search]` | List/search conversation threads |
| `neo resume <thread-id>` | Resume a previous conversation |
| `neo cost [--period]` | Show cost summary |
| `neo models [--sort]` | List available OpenRouter models |

---

## Agents

Neo routes each request to the most appropriate agent:

| Agent | Role | Tools |
|-------|------|-------|
| **Router** | Classifies requests, selects the right agent | — |
| **Planner** | Decomposes complex tasks into ordered sub-tasks | read, grep, glob, git_log |
| **Coder** | Writes and edits code | read, edit, create, grep, glob, bash, git_diff |
| **Reviewer** | Reviews code for correctness, security, performance | read, grep, glob, git_diff |
| **Debugger** | Diagnoses failures, finds root causes | read, grep, bash, git_diff, git_log |
| **Tester** | Generates and runs tests | read, edit, create, bash, grep |
| **Documenter** | Writes documentation and changelogs | read, grep, glob, git_log |
| **Oracle** | Deep architectural analysis and guidance | read, grep, glob |

---

## Model Routing

Neo selects the optimal model for each task using a weighted scoring algorithm:

| Factor | Weight | What it measures |
|--------|--------|------------------|
| Capability match | 40% | Model strengths vs. task category + coding/reasoning scores |
| Cost efficiency | 25% | $/token relative to budget preferences |
| Latency | 20% | Speed tier vs. task urgency |
| Context fit | 15% | Context window appropriateness (not too small, not wastefully large) |

Default model matrix includes Claude Sonnet, GPT-4o, o3, DeepSeek V3, Gemini 2.5 Pro, and Haiku — all accessed through OpenRouter.

---

## Configuration

```bash
# Show current config
neo config

# Set values
neo config set default_model anthropic/claude-sonnet-4-20250514
neo config set budget.max_per_day 10.00
```

Config files (highest precedence first):
1. `<workspace>/.neo/local.toml` — Personal workspace overrides (gitignored)
2. `<workspace>/.neo/config.toml` — Shared workspace config
3. `~/.config/neo/config.toml` — Global defaults

Environment variables override all files: `NEO_DEFAULT_MODEL`, `NEO_BUDGET_MAX_PER_DAY`, etc.

---

## Budget Controls

```toml
[budget]
max_per_request = 0.50    # USD hard cap per model call
max_per_session = 5.00    # USD hard cap per REPL session
max_per_day = 20.00       # USD rolling 24h cap
warn_at_percentage = 80   # warn when approaching limit
```

Each response shows cost: `model: claude-sonnet-4 | tokens: 1,247 in / 892 out | cost: $0.012 | session: $0.34`

---

## Project Structure

```
neo/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI dispatch
│   ├── lib.rs               # Module declarations
│   ├── cli/                 # Clap commands + interactive REPL
│   ├── config/              # Layered TOML config + env overrides
│   ├── api/                 # OpenRouter client (streaming, retry)
│   ├── router/              # Model selection algorithm
│   ├── agents/              # 8 agent definitions + executor loop
│   ├── tools/               # 7 sandboxed tools
│   ├── session/             # Thread persistence + cost tracking
│   └── orchestrator/        # Request classification + agent dispatch
├── SPEC.md                  # Full design specification
└── README.md
```

---

## License

MIT
