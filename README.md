# ollama-controls

Rust workspace for managing **[Ollama](https://ollama.com)** models: a **`ollama-controls`** library (CLI wrappers + Ollama HTTP client + local daemon helpers) and an **`ollama-controls-api`** HTTP gateway so you can drive Ollama from another machine or UI.

## Prerequisites

- **Rust** (stable toolchain, recent enough for your `edition` in `Cargo.toml`)
- **[Ollama](https://ollama.com)** installed and on `PATH` for CLI-backed features (`ollama list`, `ollama show`, `ollama serve`, etc.)
- For the REST gateway: nothing beyond `cargo run` (Tokio + Axum)

## Repository layout

| Crate | Purpose |
|--------|---------|
| [`ollama-controls`](ollama-controls/) | Library: `OllamaClient` (remote Ollama HTTP API), `config` (models directory + start/stop `ollama serve`), CLI helpers, `Model` / `ModelDetails` parsing |
| [`ollama-controls-api`](ollama-controls-api/) | Axum server exposing those capabilities as JSON over HTTP |

Build everything from the workspace root:

```bash
cd ollama-controls   # workspace root containing both crates
cargo build
cargo test
```

Run the HTTP API:

```bash
cargo run -p ollama-controls-api
```

Use `-p` from the **workspace root** so the path dependency `../ollama-controls` resolves correctly.

---

## Library: `ollama-controls`

Add to your `Cargo.toml`:

```toml
ollama-controls = { path = "../ollama-controls" }  # or git / crates.io when published
```

### Remote Ollama via HTTP (`OllamaClient`)

[`OllamaClient`](ollama-controls/src/api.rs) talks to Ollama’s REST API (default base URL `http://127.0.0.1:11434`). Aligns with the same **`OLLAMA_HOST`** convention as the `ollama` CLI.

**Construction**

- `OllamaClient::new("http://host:11434")`
- `OllamaClient::localhost()`
- `OllamaClient::from_env()` — uses `OLLAMA_HOST` when set, otherwise localhost

**Typical methods**

| Method | Role |
|--------|------|
| `list_models()` | `GET /api/tags` |
| `show(name)` | `POST /api/show` |
| `pull_blocking` / `pull_stream` | `POST /api/pull` |
| `delete_model` | `DELETE /api/delete` |
| `copy_model` | `POST /api/copy` |
| `create_model` / `create_from_base` | `POST /api/create` (Modelfile parsing matches Ollama 0.21+ `from` + optional `modelfile`) |
| `list_running` | `GET /api/ps` |
| `unload_model` | `POST /api/generate` with empty prompt and `keep_alive: 0` |
| `update_all_models` | Re-pull every installed tag |
| `remove_all_except` | Delete all tags not in a keep list |

Helpers: [`ollama_base_url`](ollama-controls/src/api.rs), [`split_modelfile_from`](ollama-controls/src/api.rs), [`DEFAULT_OLLAMA_PORT`](ollama-controls/src/api.rs).

### Local CLI wrappers

These invoke the **`ollama`** binary (same as typing commands in a shell):

| Function | CLI equivalent |
|----------|----------------|
| `list_models` / `list_local_downloaded_models` | `ollama list` (lines of output) |
| `list_running_models` | `ollama ps` |
| `download_model` | `ollama pull` using [`Model::get_extended_name`](ollama-controls/src/lib.rs) |
| `inspect_model` | `ollama show` (raw text) |
| `inspect_model_details` | `ollama show` + parse into [`ModelDetails`](ollama-controls/src/lib.rs) |
| `to_model_details` | Parse already-fetched show text |
| `ollama_pull` | `ollama pull <tag>` |
| `ollama_rm` | `ollama rm` |
| `ollama_cp` | `ollama cp` |
| `ollama_create_from_file` | `ollama create -f` |
| `ollama_update_all_installed` | Re-pull every model from `ollama list` |

### Config: models directory and `ollama serve`

Ollama stores models under a directory controlled by the **`OLLAMA_MODELS`** environment variable (see Ollama docs). This crate persists an optional path in:

- **Unix:** `~/.config/ollama-controls/settings.json` (or `$XDG_CONFIG_HOME/ollama-controls/` when set)
- **Windows:** `%APPDATA%\ollama-controls\settings.json`

| Function | Role |
|----------|------|
| `models_path_info()` | Current env, saved path, default `~/.ollama/models`, suggested `export` line |
| `set_models_download_path(path)` | Create directory if needed and save to `settings.json` |
| `ollama_start_serve()` | Run `ollama serve` in the background; apply saved `OLLAMA_MODELS`; write PID file for stop |
| `ollama_stop_serve()` | Stop using saved PID, then best-effort `pkill` / Windows `taskkill` |

**Important:** If Ollama runs as a **system service** or **desktop app**, changing the saved path only affects processes **you** start with `ollama_start_serve` until you also set `OLLAMA_MODELS` for that service and restart it. Moving data between disks is covered in Ollama’s documentation; this crate does not move blobs for you.

### Types

- [`Model`](ollama-controls/src/lib.rs) — identity fields for pull/show; optional [`ModelDetails`](ollama-controls/src/lib.rs) after inspect.
- API DTOs: [`ListedModel`](ollama-controls/src/api.rs), [`ShowResponse`](ollama-controls/src/api.rs), [`PullProgressLine`](ollama-controls/src/api.rs), etc.

---

## HTTP gateway: `ollama-controls-api`

### Environment

| Variable | Default | Meaning |
|----------|---------|---------|
| `OLLAMA_CONTROLS_BIND` | `127.0.0.1:3000` | Address the **gateway** listens on |
| `OLLAMA_HOST` | (unset → client uses `127.0.0.1:11434`) | Ollama server URL for **API** routes that call `OllamaClient` |
| `OLLAMA_MODELS` | (unset) | Inherited by child processes; also reflected in settings introspection |

### Security

The gateway does **not** implement authentication. Bind to **`127.0.0.1`** (default) for local-only use, or place it behind a reverse proxy (TLS, API keys, VPN) before exposing it on a network.

### Endpoints

All JSON bodies use `Content-Type: application/json` unless noted.

#### Health

| Method | Path | Response |
|--------|------|----------|
| GET | `/health` | `{ "ok": true }` |

#### Ollama HTTP API (via `OllamaClient`)

| Method | Path | Notes |
|--------|------|--------|
| GET | `/api/tags` | Installed models (`GET /api/tags` on Ollama) |
| GET | `/api/ps` | Loaded models |
| GET | `/api/show?name=<tag>` | Query param **required** |
| POST | `/api/pull` | Body: `{ "name": "model:tag" }` — returns `{ "lines": [ … ] }` pull progress objects |
| DELETE | `/api/delete?name=<tag>` | |
| POST | `/api/copy` | Body: `{ "source": "…", "destination": "…" }` |
| POST | `/api/create` | Body: `{ "name": "…", "modelfile": "FROM …\n…" }` |
| POST | `/api/create-from-base` | Body: `{ "new_name": "…", "from": "…" }` |
| POST | `/api/unload` | Body: `{ "model": "…" }` |
| POST | `/api/update-all` | Re-pull all tags |
| POST | `/api/remove-except` | Body: `{ "keep": [ "tag1", "tag2" ] }` |

#### CLI-backed inspect

| Method | Path | Notes |
|--------|------|--------|
| GET | `/api/inspect/raw?name=<tag>` | Plain text body: `ollama show` output |
| GET | `/api/inspect/details?name=<tag>` | JSON [`ModelDetails`](ollama-controls/src/lib.rs) |

#### Local service and paths

| Method | Path | Notes |
|--------|------|--------|
| POST | `/api/service/start` | Start `ollama serve` — `{ "pid": <u32> }` |
| POST | `/api/service/stop` | Stop — `204 No Content` |
| GET | `/api/local/models` | JSON array of strings: lines from `ollama list` |
| GET | `/api/settings/models-path` | [`ModelsPathInfo`](ollama-controls/src/config.rs) |
| POST | `/api/settings/models-path` | Body: `{ "path": "/absolute/or/relative/path" }` — saves and returns updated info |

### Example: curl

```bash
# Health
curl -s http://127.0.0.1:3000/health

# List models (Ollama HTTP API)
curl -s http://127.0.0.1:3000/api/tags

# List models (CLI `ollama list`)
curl -s http://127.0.0.1:3000/api/local/models

# Show manifest-style info (Ollama HTTP API)
curl -s 'http://127.0.0.1:3000/api/show?name=llama3.2:latest'

# Pull (wait for stream to finish; response contains progress lines)
curl -s -X POST http://127.0.0.1:3000/api/pull \
  -H 'Content-Type: application/json' \
  -d '{"name":"llama3.2:latest"}'

# Set download directory for future ollama_start_serve (creates dir if needed)
curl -s -X POST http://127.0.0.1:3000/api/settings/models-path \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/ollama-models"}'

# Start / stop local ollama serve (uses saved models path)
curl -s -X POST http://127.0.0.1:3000/api/service/start
curl -s -X POST http://127.0.0.1:3000/api/service/stop
```

Point **`OLLAMA_HOST`** at a remote Ollama when the gateway runs on another host:

```bash
OLLAMA_HOST=http://192.168.1.10:11434 cargo run -p ollama-controls-api
```

---

## License

Add a `LICENSE` file if you distribute this project; the README does not assume one.

## Contributing

Issues and PRs welcome; keep changes focused and match existing style in the crate you touch.
