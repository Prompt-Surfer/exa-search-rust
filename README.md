# openclaw-exa-skill

OpenClaw [skill](https://openclaw.ai/docs/skills) for [Exa AI](https://exa.ai) search — neural search, find similar, and page contents via a native Rust binary.

> **Looking for the OpenClaw plugin version?** → [openclaw-exa-plugin](https://github.com/Prompt-Surfer/openclaw-exa-plugin) (registers `web_search_exa` as a native tool, no Bash required)

---

## Plugin vs Skill

| | [Plugin](https://github.com/Prompt-Surfer/openclaw-exa-plugin) | Skill (this repo) |
|---|---|---|
| How agent calls it | Native `web_search_exa` tool | `Bash` tool invocation |
| Requires Bash access | No | Yes |
| Gateway restart to update | Yes | Never |
| ClawHub compatible | Harder | ✅ Easy |
| Sandbox compatible | ✅ | No |
| Zero config code | No | ✅ |

Use the **plugin** if you want a clean native tool. Use this **skill** if you want simpler distribution, ClawHub compatibility, or to avoid the TypeScript plugin layer.

---

## Install

```bash
bash install.sh
```

Builds the Rust binary from [openclaw-exa-plugin](https://github.com/Prompt-Surfer/openclaw-exa-plugin) (local source or auto-cloned) and installs to `~/.openclaw/workspace/skills/exa-search/`.

Then add your API key to `~/.openclaw/workspace/.env`:

```bash
echo "EXA_API_KEY=your_key_here" >> ~/.openclaw/workspace/.env
```

---

## Usage

Agents read `SKILL.md` for full instructions. Quick reference:

```bash
# Search
echo '{"query":"rust async programming","num_results":5,"livecrawl":"never"}' \
  | EXA_API_KEY=$(grep EXA_API_KEY ~/.openclaw/workspace/.env | cut -d= -f2) \
  ~/.openclaw/workspace/skills/exa-search/bin/exa-search | jq .

# Find similar pages
echo '{"action":"find_similar","url":"https://doc.rust-lang.org","num_results":3}' \
  | EXA_API_KEY=... ~/.openclaw/workspace/skills/exa-search/bin/exa-search | jq .

# Fetch page contents
echo '{"action":"get_contents","urls":["https://example.com"]}' \
  | EXA_API_KEY=... ~/.openclaw/workspace/skills/exa-search/bin/exa-search | jq .
```

---

## Benchmark

| Mode | Avg | Peak |
|---|---|---|
| Instant (`livecrawl: "never"`) | ~440ms | **308ms** |
| Default | ~927ms | 629ms |
| Exa MCP (`npx` cached) | ~5,747ms | — |

**Peak 308ms — 18.7× faster than MCP.** See [openclaw-exa-plugin](https://github.com/Prompt-Surfer/openclaw-exa-plugin#benchmark) for full benchmark table.

---

## Structure

```
.
├── SKILL.md          # Agent instructions — actions, params, output format
├── install.sh        # Build binary from bundled source + install to workspace
├── Cargo.toml        # Rust crate
├── src/              # Rust source — self-contained, no external repo needed
│   ├── main.rs
│   ├── client.rs
│   ├── protocol.rs
│   └── types/
└── bin/
    └── exa-search    # Pre-built Linux x86_64 binary (also buildable via install.sh)
```
