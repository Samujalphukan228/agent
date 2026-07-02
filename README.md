# godmode

**This is the foundation of luna cli**

Local terminal AI agent with shell, file, and directory tools. Runs on your machine — you provide a Google Gemini API key.

## Features

- Full-screen terminal UI ([ratatui](https://github.com/ratatui-org/ratatui))
- Google Gemini function calling (`gemini-2.5-flash`)
- 12 local tools: shell, files, search, grep, edit, delete, mkdir, move, HTTP, env info ([tools reference](tools.md))
- Paired question/answer chat layout with history sidebar
- Slash commands for settings (`/token`, `/mode`, `/history`, etc.)
- Two modes:
  - **base** — harmful actions require `y/n` approval
  - **god** — unrestricted, no confirmations

## Requirements

- Rust (stable)
- A [Google Gemini API key](https://aistudio.google.com/apikey)

## Install

```bash
git clone https://github.com/samujal/godmode.git
cd godmode
cargo build --release
```

Binary: `target/release/godmode`

## Quick start

```bash
cargo run
```

On first launch, set your token:

```
/token <your-gemini-api-key>
```

Switch mode:

```
/mode base    # safe — approvals for harmful actions
/mode god     # unrestricted
```

## Slash commands

| Command | Description |
|---------|-------------|
| `/token <key>` | Save your API token |
| `/mode base` | Safe mode — harmful actions need approval |
| `/mode god` | Unrestricted mode |
| `/mode` | Show current mode |
| `/settings` | Show token (masked) and mode |
| `/history` | Toggle question history sidebar |
| `/history open` | Open history sidebar |
| `/history close` | Close history sidebar |
| `/clear` | Clear conversation |
| `/help` | List all commands |

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+Enter / Alt+Enter | New line in input |
| ↑ / ↓, j / k | Scroll chat (when input empty) |
| Page Up / Page Down | Page scroll |
| h / Ctrl+B | Toggle history sidebar |
| y / n | Approve / deny (base mode) |
| Ctrl+C / Ctrl+Q | Quit |

## Configuration

Settings are saved to `~/.config/godmode/config.toml`.

Optional `.env` fallback (see `.env.example`):

```bash
cp .env.example .env
# edit GEMINI_API_KEY=
```

## Project structure

```
.
├── src/
│   ├── main.rs          # event loop
│   ├── agent.rs         # tool dispatch + history
│   ├── commands.rs      # slash commands
│   ├── mode.rs          # base/god + approvals
│   ├── settings.rs      # config persistence
│   ├── ui/              # TUI
│   ├── llm/             # Gemini client
│   └── tools/           # local tool handlers
├── tools.md             # tool reference
└── README.md
```

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo run
```

CI runs format check, clippy, and release build on push/PR.

## License

[MIT](LICENSE)