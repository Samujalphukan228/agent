# godmode — Tools Reference

The agent exposes **12 tools** to the model via Gemini function calling. All tools run locally on your machine.

| # | Tool | Module | Purpose |
|---|------|--------|---------|
| 1 | `run_shell` | `src/tools/shell.rs` | Execute shell commands |
| 2 | `read_file` | `src/tools/file.rs` | Read a file |
| 3 | `write_file` | `src/tools/file.rs` | Create or overwrite a file |
| 4 | `list_dir` | `src/tools/file.rs` | List directory entries |
| 5 | `search_files` | `src/tools/search.rs` | Find files by glob pattern |
| 6 | `grep` | `src/tools/grep.rs` | Search text in files/directories |
| 7 | `edit_file` | `src/tools/file.rs` | Replace text in a file |
| 8 | `delete_file` | `src/tools/file.rs` | Delete file or directory |
| 9 | `create_dir` | `src/tools/file.rs` | Create directory (`mkdir -p`) |
| 10 | `move_file` | `src/tools/file.rs` | Move or rename |
| 11 | `http_get` | `src/tools/http.rs` | Fetch a URL |
| 12 | `env_info` | `src/tools/env.rs` | OS / memory / disk info |

Declarations: `src/llm/client.rs` · Dispatch: `src/agent.rs` · Approval: `src/mode.rs`

---

## 1. `run_shell`

| Field | Value |
|-------|-------|
| **Parameters** | `command` (string, required) |
| **Linux** | `sh -c "<command>"` |
| **Windows** | `cmd /C "<command>"` |
| **Output** | stdout + stderr; `[exit code: N]` on failure |

---

## 2. `read_file`

| Field | Value |
|-------|-------|
| **Parameters** | `path` (string, required) |
| **Output** | Full file text |

---

## 3. `write_file`

| Field | Value |
|-------|-------|
| **Parameters** | `path`, `content` (required) |
| **Behavior** | Creates parent dirs if needed; overwrites existing |

---

## 4. `list_dir`

| Field | Value |
|-------|-------|
| **Parameters** | `path` (string, required) |
| **Output** | Sorted filenames, one per line |

---

## 5. `search_files`

| Field | Value |
|-------|-------|
| **Parameters** | `root`, `pattern` (required); `max_depth` (optional, default 8) |
| **Pattern** | Glob on filenames, e.g. `*.rs`, `Cargo.toml` |
| **Limit** | 200 results max |

**Example:**
```json
{ "root": "/home/user/project", "pattern": "*.rs", "max_depth": 6 }
```

---

## 6. `grep`

| Field | Value |
|-------|-------|
| **Parameters** | `path`, `pattern` (required); `ignore_case` (optional) |
| **Path** | Single file or directory (recursive, depth 12) |
| **Output** | `path:line:content` per match |
| **Limits** | 500 matches; skips files > 2 MB |

**Example:**
```json
{ "path": "/home/user/project/src", "pattern": "fn main", "ignore_case": false }
```

---

## 7. `edit_file`

| Field | Value |
|-------|-------|
| **Parameters** | `path`, `old_text`, `new_text` (required); `replace_all` (optional) |
| **Behavior** | First occurrence by default; `replace_all: true` for all |

**Example:**
```json
{
  "path": "/home/user/app.rs",
  "old_text": "old_fn()",
  "new_text": "new_fn()",
  "replace_all": false
}
```

---

## 8. `delete_file`

| Field | Value |
|-------|-------|
| **Parameters** | `path` (string, required) |
| **Behavior** | `remove_file` or `remove_dir_all` |

---

## 9. `create_dir`

| Field | Value |
|-------|-------|
| **Parameters** | `path` (string, required) |
| **Behavior** | `create_dir_all` (like `mkdir -p`) |

---

## 10. `move_file`

| Field | Value |
|-------|-------|
| **Parameters** | `from`, `to` (required) |
| **Behavior** | `rename`; creates parent of `to` if needed |

---

## 11. `http_get`

| Field | Value |
|-------|-------|
| **Parameters** | `url` (required, must be `http://` or `https://`) |
| **Timeout** | 30 seconds |
| **Output** | Status, headers, body (body capped at 50 KB) |

**Example:**
```json
{ "url": "https://example.com" }
```

---

## 12. `env_info`

| Field | Value |
|-------|-------|
| **Parameters** | none |
| **Output** | os, arch, family, user, home, shell, cwd, hostname, memory, disk |

---

## Output limits

| Limit | Value |
|-------|-------|
| Max output to model | **12,000 chars** (`MAX_TOOL_OUTPUT`) |
| UI preview | **16 lines** per tool block |
| `search_files` | 200 files |
| `grep` | 500 matches |
| `http_get` body | 50 KB before agent truncation |

---

## Approval (`/mode base`)

| Tool | Approval trigger |
|------|------------------|
| `run_shell` | Harmful commands (rm, sudo, shutdown, etc.) |
| `write_file`, `edit_file`, `create_dir` | Sensitive paths or `..` in path |
| `delete_file` | **Always** requires approval |
| `move_file` | Sensitive or risky `from`/`to` paths |
| `read_file`, `list_dir`, `grep`, `search_files` | Sensitive paths |
| `http_get` | **Always** requires approval |
| `env_info` | No approval |

Sensitive paths: `/etc/`, `/boot/`, `/dev/`, `/.ssh`, `/.gnupg`, system binaries, etc.

---

## Source layout

```
src/tools/
├── mod.rs
├── shell.rs      # run_shell
├── file.rs       # read, write, list, edit, delete, create_dir, move
├── search.rs     # search_files
├── grep.rs       # grep
├── http.rs       # http_get
└── env.rs        # env_info
```

---

## Adding a new tool

1. Handler in `src/tools/`
2. `FunctionDeclaration` in `src/llm/client.rs` → `tool_declarations()`
3. `execute_tool()` + `format_tool_args()` in `src/agent.rs`
4. `needs_approval()` in `src/mode.rs` if needed
5. Document here in `tools.md`