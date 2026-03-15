# Neo — Agentic AI Development Platform

## Specification v0.1

---

## 1. Overview

Neo is a CLI-first agentic AI platform for software engineering. It orchestrates multiple specialised AI agents across different models, using **OpenRouter** as a unified gateway to dynamically select the strongest model for each sub-task. Neo treats model selection as a first-class concern — not a config knob, but an active decision made per-task based on capability, cost, latency, and context requirements.

### Design Philosophy

1. **Model-agnostic by default** — No vendor lock-in. OpenRouter provides access to 200+ models; Neo picks the right one for the job.
2. **Agents are composable** — Each agent has a single responsibility (code, review, test, plan, debug). Agents delegate to other agents, forming execution graphs.
3. **CLI-native** — Designed for terminals, pipelines, and automation. No Electron. No browser tab.
4. **Developer-workflow-first** — Understands git, CI, test runners, linters, and build systems natively. Ships with opinions about how software gets built.
5. **Transparent costs** — Every model invocation shows token count, model used, and cost. No hidden spend.

---

## 2. Goals

- Provide a CLI tool that developers use like `git` — a verb-based command structure for AI-assisted development
- Dynamically route tasks to the optimal model via OpenRouter based on task type, complexity, and user budget constraints
- Support multi-agent orchestration where specialised agents collaborate on complex tasks (plan → implement → test → review)
- Integrate deeply with development workflows: git, CI/CD, test suites, linters, LSP diagnostics
- Maintain full conversation context within a session, with persistent thread history across sessions
- Run locally with no server component — API keys are the only external dependency
- Support both interactive (REPL) and non-interactive (scripted/piped) modes

## Non-Goals

- GUI or web interface (CLI only; TUI dashboards are in-scope)
- Hosting or managing models locally (use Ollama integration for local models, but Neo itself is a client)
- Replacing CI/CD systems (Neo invokes them, doesn't replace them)
- Real-time collaboration (single-user tool; concurrent agents, not concurrent humans)

---

## 3. Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        CLI Interface                         │
│              (clap commands + interactive REPL)               │
├──────────────────────────────────────────────────────────────┤
│                      Session Manager                         │
│         (thread persistence, context window mgmt)            │
├──────────────────────────────────────────────────────────────┤
│                     Orchestrator                             │
│          (agent graph, task decomposition, routing)           │
├─────────────┬──────────────┬──────────────┬─────────────────┤
│  Planner    │  Coder       │  Reviewer    │  Debugger       │
│  Agent      │  Agent       │  Agent       │  Agent          │
├─────────────┴──────────────┴──────────────┴─────────────────┤
│                     Model Router                             │
│      (OpenRouter API, model selection, cost tracking)        │
├──────────────────────────────────────────────────────────────┤
│                     Tool Layer                               │
│   (file I/O, grep, git, shell, LSP, test runner, browser)   │
└──────────────────────────────────────────────────────────────┘
```

### 3.1 Component Breakdown

| Component | Responsibility |
|-----------|---------------|
| **CLI Interface** | Parses commands, manages interactive REPL session, handles stdin/stdout piping |
| **Session Manager** | Persists conversation threads to disk, manages context windows, handles thread branching and resumption |
| **Orchestrator** | Decomposes complex tasks into agent sub-tasks, manages execution order and dependencies, aggregates results |
| **Agents** | Specialised AI actors with distinct system prompts, tool access, and model preferences |
| **Model Router** | Selects the optimal OpenRouter model per request based on task profile, enforces budget constraints |
| **Tool Layer** | Sandboxed tool implementations that agents invoke (file read/write, shell exec, git operations, etc.) |

---

## 4. Model Router — OpenRouter Integration

The Model Router is the central decision engine for model selection. It uses OpenRouter's `/api/v1/models` metadata combined with internal heuristics to match tasks to models.

### 4.1 Model Selection Strategy

Each agent request includes a **task profile** that the router uses for selection:

```rust
pub struct TaskProfile {
    pub category: TaskCategory,       // code_generation, review, planning, debugging, search
    pub estimated_complexity: Complexity, // low, medium, high, extreme
    pub context_tokens: usize,        // how much context the task needs
    pub output_expectation: OutputSize, // short (< 500 tok), medium, long, very_long
    pub latency_sensitivity: Latency, // realtime, interactive, batch
    pub requires_tool_use: bool,      // does the model need function calling support
    pub language: Option<String>,     // programming language context (for benchmark matching)
}

pub enum TaskCategory {
    CodeGeneration,   // writing new code
    CodeEdit,         // modifying existing code
    Review,           // code review, finding bugs
    Planning,         // architecture, task decomposition
    Debugging,        // root cause analysis, fix suggestions
    Search,           // codebase understanding, semantic search
    Documentation,    // writing docs, comments, READMEs
    TestGeneration,   // writing tests
    Conversation,     // general Q&A, clarification
}
```

### 4.2 Model Ranking Algorithm

For each request, the router:

1. **Filters** — Exclude models that don't meet hard constraints:
   - Context window ≥ `context_tokens`
   - Supports function calling if `requires_tool_use == true`
   - Not in user's blocklist
   - Available (OpenRouter status)

2. **Scores** — Rank remaining models on a weighted composite:

   | Factor | Weight | Source |
   |--------|--------|--------|
   | Capability match | 40% | Internal benchmark matrix (coding benchmarks per language, reasoning scores) |
   | Cost efficiency | 25% | OpenRouter pricing ($/1M tokens) relative to budget |
   | Latency | 20% | OpenRouter reported latency + historical P50 from session |
   | Context utilisation | 15% | Prefer models whose context window is ≥ needed but not excessively oversized (cost waste) |

3. **Selects** — Pick the top-ranked model, with fallback to rank #2 and #3 on failure.

### 4.3 Model Capability Matrix

Maintained as a local TOML file (`~/.config/neo/models.toml`) that ships with sensible defaults and auto-updates from OpenRouter metadata:

```toml
[models.anthropic-claude-sonnet-4]
strengths = ["code_generation", "code_edit", "review", "debugging", "tool_use"]
context = 200000
coding_score = 95     # normalized 0-100
reasoning_score = 92
speed_tier = "fast"   # fast, medium, slow

[models.openai-o3]
strengths = ["planning", "debugging", "review", "reasoning"]
context = 200000
coding_score = 90
reasoning_score = 98
speed_tier = "slow"

[models.deepseek-v3]
strengths = ["code_generation", "code_edit"]
context = 128000
coding_score = 88
reasoning_score = 82
speed_tier = "fast"
cost_tier = "budget"
```

### 4.4 Budget Controls

```toml
# ~/.config/neo/config.toml
[budget]
max_per_request = 0.50        # USD, hard cap per single model call
max_per_session = 5.00        # USD, hard cap per interactive session
max_per_day = 20.00           # USD, rolling 24h cap
warn_at_percentage = 80       # warn when approaching limit
preferred_cost_tier = "any"   # "budget", "mid", "premium", "any"
```

When a budget limit is hit, Neo:
1. Warns the user with cost-so-far and projected cost
2. Offers to downgrade to a cheaper model
3. Hard-stops if the user declines (never silently overspend)

### 4.5 OpenRouter API Integration

```
POST https://openrouter.ai/api/v1/chat/completions
Headers:
  Authorization: Bearer $OPENROUTER_API_KEY
  HTTP-Referer: https://github.com/user/neo
  X-Title: Neo CLI

Body:
  model: <selected-model-id>
  messages: [...]
  tools: [...]           # if agent needs tool use
  stream: true           # always stream for interactive mode
  temperature: <agent-specific>
  max_tokens: <task-specific>
```

Fallback chain: if the selected model returns a 5xx or rate-limit, the router automatically retries with the next-ranked model (up to 2 fallbacks) before surfacing the error.

### 4.6 Local Model Support

For offline/private use, the router also supports local Ollama endpoints:

```toml
[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
models = ["llama3.2", "codellama", "deepseek-coder"]
priority = "fallback"    # "prefer", "fallback", "disabled"
```

When `priority = "prefer"`, local models are tried first; OpenRouter is the fallback. When `priority = "fallback"`, local models are only used when OpenRouter is unavailable or budget is exhausted.

---

## 5. Agent System

### 5.1 Agent Architecture

Each agent is a stateless function: `(SystemPrompt, Messages, Tools) → (Response, ToolCalls)`. Agents don't hold state between invocations — all state lives in the Session Manager.

```rust
pub struct Agent {
    pub id: AgentId,
    pub name: &'static str,
    pub description: &'static str,
    pub system_prompt: String,
    pub available_tools: Vec<ToolId>,
    pub model_preferences: TaskProfile,   // default task profile for router
    pub max_iterations: usize,            // tool-use loop cap (prevents runaway)
    pub temperature: f32,
}
```

### 5.2 Built-in Agents

| Agent | Role | Default Model Preference | Tools |
|-------|------|--------------------------|-------|
| **Planner** | Decomposes complex tasks into ordered sub-tasks. Produces execution plans. | High reasoning, medium latency | read, grep, glob, git_log |
| **Coder** | Writes and edits code. The workhorse agent. | High coding score, tool use required | read, edit, create, grep, glob, bash, diagnostics |
| **Reviewer** | Reviews code changes for correctness, style, security, and performance. | High reasoning, medium coding | read, grep, glob, git_diff |
| **Debugger** | Diagnoses failures from error messages, logs, and test output. Root-cause analysis. | High reasoning, high coding | read, grep, bash, diagnostics, test_runner |
| **Tester** | Generates and runs tests. Validates implementations against requirements. | High coding, tool use required | read, edit, create, bash, test_runner |
| **Documenter** | Writes documentation, READMEs, inline comments, and changelogs. | Medium coding, fast latency | read, grep, glob, git_log |
| **Oracle** | Deep analysis and architectural guidance. Consulted by other agents for hard problems. | Highest reasoning (o3-class), slow latency OK | read, grep, glob, web_search |
| **Router** | Meta-agent that decides which agent(s) to invoke for a user request. Lightweight. | Fast, cheap model | none (decision only) |

### 5.3 Agent Execution Model

```
User Request
     │
     ▼
  ┌─────────┐
  │  Router  │ ── classifies request, selects agent(s)
  └────┬─────┘
       │
       ▼
  ┌──────────┐    ┌──────────┐
  │ Planner  │───▶│  Coder   │───┐
  └──────────┘    └──────────┘   │
                                  ▼
                            ┌──────────┐
                            │ Reviewer  │
                            └────┬─────┘
                                 │
                        ┌────────┴────────┐
                        │ Pass            │ Fail
                        ▼                 ▼
                    Complete         Back to Coder
                                   (with feedback)
```

**Simple requests** (single-file edit, question) go directly to the Coder agent — no Planner, no review loop.

**Complex requests** (multi-file feature, refactor) trigger the full pipeline:
1. **Planner** decomposes into sub-tasks with dependency ordering
2. **Coder** executes each sub-task (parallelised where dependencies allow)
3. **Reviewer** evaluates the aggregate diff
4. If the Reviewer rejects, the Coder receives the feedback and iterates (max 3 cycles)
5. **Tester** runs relevant tests and reports results

The Orchestrator manages this graph. Agents communicate only through the Orchestrator — never directly.

### 5.4 Agent Tool-Use Loop

Each agent runs in a loop:

```
1. Send (system_prompt + messages + tools) to model via Router
2. If response contains tool_calls:
   a. Execute each tool call (sandboxed)
   b. Append tool results to messages
   c. Go to 1 (up to max_iterations)
3. If response is text-only:
   a. Return response to Orchestrator
```

`max_iterations` prevents runaway loops. Default: 25 for Coder, 10 for Reviewer, 5 for Planner.

---

## 6. Tool Layer

### 6.1 Built-in Tools

| Tool | Description | Sandboxing |
|------|-------------|------------|
| `read` | Read file contents (with line ranges) | Read-only, workspace-scoped |
| `edit` | Apply targeted edits to existing files (old_str → new_str) | Write, workspace-scoped |
| `create` | Create new files | Write, workspace-scoped |
| `grep` | Regex search across files (ripgrep backend) | Read-only |
| `glob` | Find files by pattern | Read-only |
| `bash` | Execute shell commands | Configurable: allow-list, deny-list, or confirmation prompt |
| `git_diff` | Show uncommitted changes | Read-only |
| `git_log` | Show commit history | Read-only |
| `git_commit` | Stage and commit changes | Write, requires user confirmation |
| `test_runner` | Run test suites, parse results | Shell exec |
| `diagnostics` | Get LSP diagnostics (errors, warnings) for a file | Read-only |
| `web_search` | Search the web for documentation/answers | Network, rate-limited |
| `read_url` | Fetch and extract content from a URL | Network, rate-limited |

### 6.2 Tool Permissions

Tools are grouped into permission tiers:

```toml
# ~/.config/neo/config.toml
[permissions]
# "auto" = execute without asking
# "confirm" = ask before each execution
# "deny" = never allow
read_tools = "auto"           # read, grep, glob, git_diff, git_log, diagnostics
write_tools = "auto"          # edit, create
shell_tools = "confirm"       # bash
git_write_tools = "confirm"   # git_commit
network_tools = "auto"        # web_search, read_url
```

### 6.3 Workspace Sandboxing

All file operations are scoped to the **workspace root** (detected via `.git` root, or explicit `--workspace` flag). No tool can read or write outside the workspace unless the user explicitly passes `--allow-global`.

Bash commands run with the workspace as `cwd`. A configurable deny-list blocks dangerous commands by default:

```toml
[shell]
deny_patterns = [
    "rm -rf /",
    "sudo",
    "chmod -R 777",
    "curl .* | sh",
]
```

---

## 7. CLI Interface

### 7.1 Command Structure

```
neo <command> [options] [args]
```

| Command | Description | Example |
|---------|-------------|---------|
| `neo` | Start interactive REPL session | `neo` |
| `neo ask "<prompt>"` | One-shot question (non-interactive) | `neo ask "explain this error" < error.log` |
| `neo do "<task>"` | Execute a task end-to-end | `neo do "add input validation to signup form"` |
| `neo review` | Review uncommitted changes | `neo review` |
| `neo review <commit>` | Review a specific commit | `neo review HEAD~3..HEAD` |
| `neo test` | Generate tests for changed files | `neo test` |
| `neo debug "<error>"` | Diagnose an error | `neo debug "test_auth fails with 401"` |
| `neo plan "<task>"` | Produce an implementation plan without executing | `neo plan "migrate from REST to GraphQL"` |
| `neo doc` | Generate/update documentation for changes | `neo doc` |
| `neo config` | Open/edit configuration | `neo config set budget.max_per_day 10` |
| `neo threads` | List/search past conversation threads | `neo threads --search "auth refactor"` |
| `neo resume <thread-id>` | Resume a previous conversation | `neo resume T-abc123` |
| `neo cost` | Show cost summary for current session/day/month | `neo cost --period day` |
| `neo models` | List available models with capabilities and pricing | `neo models --sort cost` |

### 7.2 Interactive REPL

The default `neo` command enters an interactive session:

```
$ neo
neo v0.1.0 | model: auto | budget: $4.20 remaining today

> add rate limiting to the /api/users endpoint

Planning... (using o3 via OpenRouter, ~$0.08)

Plan:
  1. Add rate-limit middleware to src/middleware/rate_limit.rs [new file]
  2. Wire middleware into src/routes/users.rs
  3. Add tests to tests/rate_limit_test.rs [new file]
  4. Update README.md with rate limit documentation

Execute plan? [Y/n/edit]:
```

#### REPL Features

- **Streaming output** — Responses stream token-by-token
- **Multi-line input** — Shift+Enter or `\` at line end for continuation
- **File references** — `@src/main.rs` auto-reads and attaches file content
- **Pipe input** — `cat error.log | neo ask "what went wrong"` works
- **Thread persistence** — Every session is auto-saved; resume with `neo resume`
- **Interrupt** — Ctrl+C cancels the current agent operation (not the session)
- **Cost display** — Each response footer shows: `model: claude-sonnet-4 | tokens: 1,247 in / 892 out | cost: $0.012 | session: $0.34`

### 7.3 Output Modes

| Flag | Mode | Use Case |
|------|------|----------|
| (default) | Rich terminal | Interactive use with colors, spinners, streaming |
| `--json` | JSON output | Piping to other tools, CI integration |
| `--quiet` | Minimal | Only final output, no progress/status |
| `--diff` | Unified diff | Show only the changes that would be/were made |
| `--dry-run` | Preview | Show what would happen without executing |

### 7.4 CI/CD Integration

Neo can run headless in CI pipelines:

```yaml
# GitHub Actions example
- name: AI Code Review
  run: neo review ${{ github.event.pull_request.base.sha }}..${{ github.sha }} --json
  env:
    OPENROUTER_API_KEY: ${{ secrets.OPENROUTER_API_KEY }}
    NEO_BUDGET_MAX_REQUEST: "0.50"
    NEO_PERMISSIONS_SHELL: "deny"
    NEO_PERMISSIONS_WRITE: "deny"
```

---

## 8. Session & Context Management

### 8.1 Thread Persistence

Every conversation is a **thread** with a unique ID. Threads are stored locally:

```
~/.local/share/neo/threads/
├── T-2026-03-15-a8f3c1.json
├── T-2026-03-15-b2d4e7.json
└── index.json
```

Thread format:

```rust
pub struct Thread {
    pub id: ThreadId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub workspace: PathBuf,
    pub messages: Vec<Message>,
    pub cost_total: f64,
    pub models_used: Vec<String>,
    pub tags: Vec<String>,        // auto-tagged from content
}
```

### 8.2 Context Window Management

When conversation history exceeds the model's context window:

1. **Summarise** — The oldest messages are summarised by a cheap, fast model (e.g., GPT-4o-mini) into a condensed context block
2. **Preserve recent** — The last N messages (default 20) are kept verbatim
3. **Pin** — Users can pin messages that should never be summarised (e.g., architectural decisions)
4. **Tool results** — Large tool outputs (file reads, grep results) are truncated to relevant excerpts in older messages

### 8.3 Workspace Context

On session start, Neo builds a workspace profile:

```rust
pub struct WorkspaceContext {
    pub root: PathBuf,
    pub vcs: Option<VcsInfo>,         // git branch, recent commits, dirty files
    pub language: Vec<Language>,       // detected from file extensions + config files
    pub build_system: Option<String>,  // cargo, npm, make, gradle, etc.
    pub test_framework: Option<String>,// jest, pytest, cargo test, etc.
    pub linter: Option<String>,        // eslint, clippy, ruff, etc.
    pub ci: Option<CiSystem>,          // github actions, gitlab ci, etc.
    pub conventions: Option<String>,   // from CONTRIBUTING.md, .editorconfig, etc.
    pub structure: DirectoryTree,      // top-level directory listing
}
```

This context is injected into the system prompt so agents understand the project without being told.

---

## 9. Development Workflow Optimisation

### 9.1 Git-Aware Operations

Neo understands git state and uses it to scope work:

- **Auto-scoping** — `neo review` with no args reviews uncommitted changes. `neo test` targets files changed since last commit.
- **Branch context** — System prompt includes current branch name and recent commit messages for continuity
- **Commit hygiene** — The Coder agent only stages files it modified. Never `git add .`.
- **Diff-based review** — The Reviewer agent receives unified diffs, not full files, for focused review

### 9.2 Test-Driven Workflow

When the user requests a feature, the full pipeline is:

```
1. Planner produces sub-tasks
2. Tester generates test stubs (red)
3. Coder implements until tests pass (green)
4. Reviewer checks the implementation
5. Coder refactors if needed (refactor)
```

This is opt-in via `neo do --tdd "feature description"` or configured as default:

```toml
[workflow]
default_strategy = "implement"    # "implement", "tdd", "plan_only"
auto_test = true                  # run tests after code changes
auto_lint = true                  # run linter after code changes
review_threshold = "multi_file"   # "always", "multi_file", "never"
```

### 9.3 Diagnostics Integration

After every code change, Neo automatically:

1. Runs the project's **linter** (if detected) on modified files
2. Checks **LSP diagnostics** (if available) for type errors
3. Runs **affected tests** (if test runner detected)

Failures are fed back to the Coder agent for self-correction before presenting results to the user.

### 9.4 Convention Detection

On first run in a workspace, Neo scans for:

| Signal | Source | Effect |
|--------|--------|--------|
| Code style | `.editorconfig`, `.prettierrc`, `rustfmt.toml` | Agent follows formatting conventions |
| Linting rules | `.eslintrc`, `clippy.toml`, `ruff.toml` | Agent avoids patterns the linter would flag |
| Project conventions | `CONTRIBUTING.md`, `AGENTS.md` | Injected into system prompts |
| Import style | Existing code analysis | Agent matches import ordering and grouping |
| Error handling | Existing code analysis | Agent matches `Result`/`try-catch`/`Option` patterns |
| Test patterns | Existing test files | Agent generates tests in the same style |

### 9.5 Parallel Agent Execution

For multi-file tasks, independent sub-tasks run in parallel:

```
Plan:
  Task 1: Add middleware    (src/middleware/rate_limit.rs)
  Task 2: Update API docs   (docs/api.md)
  Task 3: Add config schema (src/config/schema.rs)

Tasks 1, 2, 3 have no dependencies → execute in parallel (3 concurrent Coder agents)

  Task 4: Wire middleware into routes (depends on Task 1)
  Task 5: Add integration test (depends on Tasks 1, 3)

Tasks 4, 5 wait for dependencies → execute after prerequisites complete
```

Each parallel agent uses its own model invocation. The Orchestrator merges results and resolves conflicts (if two agents edit the same file, they run sequentially instead).

---

## 10. Configuration

### 10.1 Config File Location

```
~/.config/neo/config.toml       # global defaults
<workspace>/.neo/config.toml    # workspace overrides (committed to repo)
<workspace>/.neo/local.toml     # workspace overrides (gitignored, personal)
```

Precedence: local.toml > workspace config.toml > global config.toml > built-in defaults.

### 10.2 Full Config Schema

```toml
[core]
default_model = "auto"              # "auto" or specific model ID
interactive_model = "auto"          # model for REPL conversation
planning_model = "auto"             # model for Planner agent
temperature = 0.3                   # default temperature

[budget]
max_per_request = 0.50
max_per_session = 5.00
max_per_day = 20.00
warn_at_percentage = 80
preferred_cost_tier = "any"

[permissions]
read_tools = "auto"
write_tools = "auto"
shell_tools = "confirm"
git_write_tools = "confirm"
network_tools = "auto"

[workflow]
default_strategy = "implement"
auto_test = true
auto_lint = true
auto_format = false
review_threshold = "multi_file"
max_review_cycles = 3
parallel_agents = true

[shell]
deny_patterns = ["rm -rf /", "sudo", "chmod -R 777"]
allowed_commands = []               # empty = all allowed (minus deny)
timeout_seconds = 30

[providers.openrouter]
api_key_env = "OPENROUTER_API_KEY"  # env var name (never store key in config)
base_url = "https://openrouter.ai/api/v1"
max_retries = 3
timeout_seconds = 120

[providers.ollama]
enabled = false
endpoint = "http://localhost:11434"
models = []
priority = "fallback"

[context]
max_file_lines = 500                # truncate file reads beyond this
summary_threshold = 50000           # summarise context beyond this token count
pin_system_messages = true

[ui]
color = "auto"                      # "auto", "always", "never"
spinner = true
streaming = true
show_cost = true
show_model = true
```

### 10.3 Environment Variables

All config values can be overridden via environment variables:

```
OPENROUTER_API_KEY=sk-or-...
NEO_DEFAULT_MODEL=anthropic/claude-sonnet-4-20250514
NEO_BUDGET_MAX_PER_DAY=10.00
NEO_PERMISSIONS_SHELL=deny
NEO_WORKFLOW_AUTO_TEST=false
```

Pattern: `NEO_<SECTION>_<KEY>` in SCREAMING_SNAKE_CASE.

---

## 11. Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | **Rust** | Performance, single binary distribution, strong typing, async ecosystem |
| CLI framework | `clap` v4 | Derive-based arg parsing, shell completions, subcommand structure |
| Async runtime | `tokio` | Industry standard, required for concurrent agent execution |
| HTTP client | `reqwest` | Async, streaming, TLS, widely used |
| JSON | `serde` + `serde_json` | De facto standard |
| Config | `toml` + `serde` | Human-readable config format |
| Terminal UI | `crossterm` + `indicatif` | Cross-platform terminal manipulation, progress bars/spinners |
| File search | `ignore` + `regex` crates | Fast codebase search (same walker as ripgrep) |
| File matching | `glob` crate | Pattern-based file discovery |
| Storage | SQLite via `rusqlite` (bundled) | Thread persistence, cost tracking, model performance history |
| REPL | `rustyline` | Line editing, history, Ctrl+C handling |
| Terminal color | `colored` | ANSI color output for CLI |

### 11.1 Binary Distribution

Single static binary. No runtime dependencies (except system TLS).

```
neo-x86_64-unknown-linux-musl
neo-aarch64-apple-darwin
neo-x86_64-pc-windows-msvc.exe
```

Install methods:
- `cargo install neo-cli`
- `brew install neo-cli` (macOS/Linux)
- GitHub Releases (prebuilt binaries)
- Nix flake

### 11.2 Project Structure

```
neo/
├── Cargo.toml
├── SPEC.md
├── README.md
├── LICENSE
└── src/
    ├── main.rs                      # Entry point, CLI dispatch
    ├── lib.rs                       # Module declarations
    ├── cli/
    │   ├── commands.rs              # Clap derive subcommands (12 commands)
    │   └── repl.rs                  # Interactive REPL with rustyline
    ├── config/
    │   ├── types.rs                 # NeoConfig, BudgetConfig, Permission, etc.
    │   └── loader.rs                # Layered config loading + env var overrides
    ├── api/
    │   ├── types.rs                 # OpenRouter request/response/streaming types
    │   └── client.rs                # OpenRouterClient with retry + SSE streaming
    ├── router/
    │   ├── types.rs                 # TaskProfile, TaskCategory, Complexity
    │   ├── capabilities.rs          # ModelCapability matrix (7 models)
    │   └── selector.rs              # ModelRouter weighted scoring algorithm
    ├── agents/
    │   ├── types.rs                 # AgentId, AgentConfig
    │   ├── definitions.rs           # 8 agent definitions with system prompts
    │   └── executor.rs              # AgentExecutor tool-use loop
    ├── tools/
    │   ├── mod.rs                   # ToolRegistry, ToolKind enum dispatch
    │   ├── read_file.rs             # Read file with line ranges
    │   ├── edit_file.rs             # Targeted string replacement
    │   ├── create_file.rs           # Create new files
    │   ├── grep.rs                  # Regex search (ignore + regex crates)
    │   ├── glob_tool.rs             # File pattern matching
    │   ├── bash.rs                  # Shell execution with deny-list
    │   └── git.rs                   # git diff + git log
    ├── session/
    │   ├── types.rs                 # Thread, SessionStats
    │   └── manager.rs               # Thread persistence + cost tracking
    └── orchestrator/
        └── mod.rs                   # Request classification + agent dispatch
```

---

## 12. Security Model

### 12.1 Secrets Management

- API keys are **never** stored in config files — only env var references
- The `OPENROUTER_API_KEY` env var is the sole credential required
- Neo never logs, displays, or transmits API keys
- File contents sent to models may contain secrets — Neo warns if `.env` files or known secret patterns are detected in context

### 12.2 Sandboxing

- All file operations are workspace-scoped by default
- Shell commands run with configurable deny-lists
- Network access (web search, URL fetch) is rate-limited and logged
- No automatic privilege escalation — Neo never runs `sudo`

### 12.3 Data Privacy

- All thread history is stored locally (`~/.local/share/neo/`)
- No telemetry, no phone-home, no analytics
- Model API calls go through OpenRouter (their privacy policy applies) or local Ollama
- Users can configure `--no-history` to disable thread persistence

---

## 13. Cost Tracking & Observability

### 13.1 Per-Request Logging

Every model invocation is logged:

```rust
pub struct InvocationLog {
    pub timestamp: DateTime<Utc>,
    pub thread_id: ThreadId,
    pub agent: AgentId,
    pub model: String,
    pub provider: String,            // "openrouter" or "ollama"
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub status: InvocationStatus,    // success, error, fallback
    pub fallback_from: Option<String>, // if this was a fallback, which model failed
}
```

### 13.2 Cost Dashboard

`neo cost` shows:

```
Today:     $2.34 / $20.00 (12%)
This week: $8.91
This month: $34.20

Top models by spend:
  claude-sonnet-4       $1.80 (77%)  │ 14 calls │ avg 1.2s
  o3                    $0.42 (18%)  │  3 calls │ avg 4.8s
  gpt-4o-mini           $0.12  (5%)  │ 28 calls │ avg 0.3s

Top agents by spend:
  Coder                 $1.44 (62%)
  Planner               $0.42 (18%)
  Reviewer              $0.36 (15%)
  Router                $0.12  (5%)
```

### 13.3 Model Performance Tracking

Neo tracks per-model success rates locally to improve routing over time:

- **Success rate** — Did the agent complete its task without errors?
- **Retry rate** — How often did the model require retries or fallbacks?
- **Tool-use accuracy** — Did the model produce valid tool calls?
- **Latency P50/P95** — Actual observed latency

This data feeds back into the Model Router's scoring algorithm as a local adjustment factor.

---

## 14. Extensibility

### 14.1 Custom Agents

Users can define custom agents via TOML:

```toml
# .neo/agents/security_reviewer.toml
[agent]
name = "SecurityReviewer"
description = "Reviews code changes for security vulnerabilities"
system_prompt = """
You are a security-focused code reviewer. Analyze code for:
- Injection vulnerabilities (SQL, XSS, command injection)
- Authentication/authorization flaws
- Secret exposure
- Insecure dependencies
"""
tools = ["read", "grep", "glob", "bash"]
model_preference = "high_reasoning"
temperature = 0.2
max_iterations = 10
```

### 14.2 Custom Tools

Tools can be added as shell scripts or binaries:

```toml
# .neo/tools/run_migration.toml
[tool]
name = "run_migration"
description = "Run database migrations and return status"
command = "npm run migrate:status"
working_directory = "."
timeout_seconds = 30
permission = "confirm"

[tool.parameters]
direction = { type = "string", enum = ["up", "down"], required = true }
```

### 14.3 Workflow Presets

Pre-configured workflows for common development patterns:

```toml
# .neo/workflows/feature.toml
[workflow]
name = "feature"
description = "Full feature implementation with TDD"
steps = [
    { agent = "Planner", task = "Decompose the feature into sub-tasks" },
    { agent = "Tester", task = "Write failing tests for each sub-task" },
    { agent = "Coder", task = "Implement until tests pass", parallel = true },
    { agent = "Reviewer", task = "Review the complete changeset" },
    { agent = "Documenter", task = "Update relevant documentation" },
]
```

---

## 15. Implementation Status

### Phase 1 — Foundation ✅ Complete

- [x] Project scaffold (Rust, clap, tokio) — 31 source files, ~3,400 LOC
- [x] OpenRouter API client with streaming — SSE parsing, exponential backoff retry on 429/5xx
- [x] Model Router with weighted selection — capability (40%) / cost (25%) / latency (20%) / context fit (15%)
- [x] Core tool layer — 7 tools: `read`, `edit`, `create`, `grep`, `glob`, `bash`, `git` (diff + log)
- [x] All 8 agents implemented — Router, Planner, Coder, Reviewer, Debugger, Tester, Documenter, Oracle
- [x] Agent executor with tool-use loop — iteration-capped, per-agent model routing
- [x] Interactive REPL — rustyline, spinner, cost footer, Ctrl+C handling
- [x] Config system — layered TOML (global → workspace → local) + `NEO_*` env var overrides
- [x] Thread persistence — JSON file storage in `~/.local/share/neo/threads/`
- [x] Orchestrator — request classification, agent dispatch, cost tracking
- [x] 12 CLI subcommands — `ask`, `do`, `review`, `test`, `debug`, `plan`, `doc`, `config`, `threads`, `resume`, `cost`, `models`
- [x] Budget controls — per-request/session/day caps with configurable limits
- [x] 8.7 MB single static binary, zero warnings

### Phase 2 — Multi-Agent Orchestration ✅ Complete

- [x] Execution plan data model with dependency graph (`orchestrator/plan.rs`)
- [x] Plan parser — extracts numbered steps, agents, dependencies, and files from Planner output
- [x] Parallel group computation — topological sort into dependency-respecting batches
- [x] Review feedback loop — Reviewer → Coder iteration, max 3 cycles, auto-approval detection
- [x] Planner → Coder → Reviewer pipeline (`orchestrator/pipeline.rs`) with graceful fallback
- [x] `neo pipeline <task>` CLI subcommand (13th subcommand)
- [x] `/pipeline <task>` REPL command with spinner and pipeline status output
- [x] Context window management (automatic summarisation of old messages) — Phase 1 already delivered this
- [x] OrchestratorResponse extended with `pipeline_steps` and `review_cycles` metadata

### Phase 3 — Workflow Integration

- [ ] Workspace context detection (language, build system, test framework, linter)
- [ ] Convention detection from existing code and config files
- [ ] Auto-lint and auto-test after code changes
- [ ] Git-aware operations (diff-scoped review, branch context injection)
- [ ] CI/CD headless mode (`--json`, `--quiet` output formats)
- [ ] Shell completions (bash, zsh, fish)

### Phase 4 — Polish & Extensibility

- [ ] Custom agent definitions via TOML (`.neo/agents/*.toml`)
- [ ] Custom tool registration (`.neo/tools/*.toml`)
- [ ] Workflow presets (`.neo/workflows/*.toml`)
- [ ] Model performance tracking and adaptive routing
- [ ] Ollama/local model support with `priority = "prefer"` / `"fallback"`
- [ ] SQLite-backed cost dashboard with daily/weekly/monthly rollups
- [ ] Package and publish (crates.io, Homebrew, GitHub Releases, Nix flake)

---

## 16. Success Metrics

| Metric | Target |
|--------|--------|
| Cold start to first response | < 3 seconds |
| Model selection latency | < 100ms (local decision) |
| Single-file edit end-to-end | < 15 seconds |
| Multi-file feature (5 files) | < 2 minutes |
| Cost per typical coding session (1 hour) | < $3.00 with auto-routing |
| Cost savings vs. fixed premium model | ≥ 30% (via intelligent routing to cheaper models for simple tasks) |
| Binary size | < 20 MB |
| Zero-config first run | Works with just `OPENROUTER_API_KEY` set |
