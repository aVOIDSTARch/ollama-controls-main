use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ollama_api_client::OllamaControlsApiClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    client: OllamaControlsApiClient,
    dev_api_key: String,
}

#[derive(Deserialize)]
struct ChatSendBody {
    message: String,
    model: Option<String>,
}

#[derive(Serialize)]
struct ChatSendResponse {
    response: String,
    model: String,
}

#[derive(Deserialize)]
struct AdminLoginBody {
    password: String,
}

#[derive(Deserialize)]
struct PullBody {
    name: String,
}

#[derive(Deserialize)]
struct AdminGenerateBody {
    model: String,
    prompt: String,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let bind = std::env::var("FAIL_ACADEMY_BIND").unwrap_or_else(|_| "127.0.0.1:3005".to_string());
    let dev_api_key = std::env::var("DEV_API_KEY").unwrap_or_else(|_| "changeme-dev-key".to_string());
    let upstream_url =
        std::env::var("OLLAMA_CONTROLS_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let upstream_api_key = std::env::var("OLLAMA_CONTROLS_API_KEY")
        .ok()
        .or_else(|| Some(dev_api_key.clone()));

    let client = OllamaControlsApiClient::new(upstream_url).with_api_key_opt(upstream_api_key);
    let state = Arc::new(AppState { client, dev_api_key });

    let app = Router::new()
        .route("/", get(home_page))
        .route("/chat", get(chat_page))
        .route("/chat/send", post(chat_send))
        .route("/admin", get(admin_page))
        .route("/admin/login", post(admin_login))
        .route("/admin/api/tags", get(admin_tags))
        .route("/admin/api/pull", post(admin_pull))
        .route("/admin/api/generate", post(admin_generate))
        .nest_service("/public", ServeDir::new("fail-academy/public"))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("bind {bind}: {e}"));
    eprintln!("fail-academy listening on http://{bind}");
    axum::serve(listener, app).await.expect("server");
}

fn page_shell(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      --orange-1: #ff4500;
      --orange-2: #ff6347;
      --orange-3: #ff7f50;
      --orange-4: #ff8c00;
      --teal: #008080;
      --ink: #1d2b2b;
      --paper: #fffaf6;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Inter, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
      background: linear-gradient(120deg, var(--paper), #fff, #fff8ef);
      color: var(--ink);
    }}
    .hero {{
      background: linear-gradient(115deg, var(--orange-1), var(--orange-3), var(--orange-4), var(--teal));
      color: #fff;
      padding: 3rem 1.25rem;
    }}
    .hero h1 {{ margin: 0 0 0.5rem; font-size: 2rem; }}
    .hero p {{ margin: 0.25rem 0; max-width: 56rem; opacity: 0.95; }}
    nav {{ margin-top: 1rem; display: flex; gap: 0.75rem; flex-wrap: wrap; }}
    nav a {{
      color: #fff; text-decoration: none; border: 1px solid rgba(255,255,255,0.5);
      padding: 0.45rem 0.75rem; border-radius: 999px;
      background: rgba(255,255,255,0.15);
    }}
    .content {{ max-width: 1000px; margin: 1.25rem auto 2.5rem; padding: 0 1rem; }}
    .card {{
      background: #fff; border: 1px solid #f0e2d6; border-radius: 14px; padding: 1rem;
      box-shadow: 0 8px 28px rgba(20,20,20,0.05); margin-bottom: 1rem;
    }}
    button {{
      border: none; border-radius: 10px; padding: 0.6rem 0.9rem; cursor: pointer;
      background: var(--teal); color: white; font-weight: 600;
    }}
    input, textarea {{
      width: 100%; border: 1px solid #d8ccc2; border-radius: 10px; padding: 0.65rem;
      font: inherit; margin: 0.25rem 0 0.75rem;
    }}
    .muted {{ color: #5a6b6b; }}
    .row {{ display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }}
    @media (max-width: 760px) {{ .row {{ grid-template-columns: 1fr; }} }}
    .palette {{
      display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 0.25rem; margin-top: 0.75rem;
    }}
    .swatch {{ color: white; font-size: 0.8rem; text-align: center; padding: 0.9rem 0.2rem; border-radius: 8px; }}
  </style>
</head>
<body>
  <section class="hero">
    <h1>FAIL Academy AI</h1>
    <p>Train smarter with an AI coach powered by your local Ollama stack.</p>
    <p class="muted">Bot at <b>localhost:3005/chat</b>, admin tools at <b>localhost:3005/admin</b>.</p>
    <nav>
      <a href="/">Home</a><a href="/chat">Chat</a><a href="/admin">Admin</a>
    </nav>
  </section>
  <main class="content">{body}</main>
</body>
</html>"#
    )
}

async fn home_page() -> Html<String> {
    let body = r#"
<div class="card">
  <h2>Build Fast. Learn Hard. Ship Anyway.</h2>
  <p>
    FAIL Academy helps teams and students practice with an always-on AI tutor and operations
    console. Your assistant runs locally for privacy and speed, while the site provides a clean
    experience for learners and admins.
  </p>
  <img src="/public/images/inspo-photo.jpeg" alt="Brand palette inspiration" style="width:100%;max-width:620px;border-radius:12px;border:1px solid #eee;" />
  <div class="palette">
    <div class="swatch" style="background:#ff4500">#ff4500</div>
    <div class="swatch" style="background:#ff6347">#ff6347</div>
    <div class="swatch" style="background:#ff7f50">#ff7f50</div>
    <div class="swatch" style="background:#ff8c00">#ff8c00</div>
    <div class="swatch" style="background:#008080">#008080</div>
  </div>
</div>
<div class="row">
  <div class="card">
    <h3>For Students</h3>
    <p>Open <code>/chat</code> and ask questions, generate explanations, or practice interview prompts.</p>
  </div>
  <div class="card">
    <h3>For Admins</h3>
    <p>Open <code>/admin</code>, enter the <code>DEV_API_KEY</code> password, and access Ollama management actions.</p>
  </div>
</div>
"#;
    Html(page_shell("FAIL Academy", body))
}

async fn chat_page() -> Html<String> {
    let body = r#"
<div class="card">
  <h2>Chat Interface</h2>
  <p class="muted">This sends prompts to <code>/api/generate</code> through your local ollama-controls API.</p>
  <label>Model</label>
  <input id="model" value="llama3.2:latest" />
  <label>Your message</label>
  <textarea id="msg" rows="5" placeholder="Ask the FAIL Academy assistant..."></textarea>
  <button id="send">Send</button>
  <pre id="out" style="white-space:pre-wrap;background:#fff6ef;border:1px solid #f5dbc7;padding:0.75rem;border-radius:10px;margin-top:0.9rem;"></pre>
</div>
<script>
const btn = document.getElementById('send');
btn.addEventListener('click', async () => {
  const model = document.getElementById('model').value.trim();
  const message = document.getElementById('msg').value.trim();
  const out = document.getElementById('out');
  if (!message) return;
  out.textContent = 'Thinking...';
  try {
    const r = await fetch('/chat/send', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({model, message})
    });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'request failed');
    out.textContent = data.response;
  } catch (e) {
    out.textContent = 'Error: ' + e.message;
  }
});
</script>
"#;
    Html(page_shell("FAIL Academy Chat", body))
}

async fn chat_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatSendBody>,
) -> Result<Json<ChatSendResponse>, (StatusCode, Json<serde_json::Value>)> {
    let model = body.model.unwrap_or_else(|| "llama3.2:latest".to_string());
    match state.client.generate(&model, &body.message) {
        Ok(r) => Ok(Json(ChatSendResponse {
            response: r.response,
            model: r.model,
        })),
        Err(e) => Err((StatusCode::BAD_GATEWAY, Json(json!({ "error": e })))),
    }
}

fn has_admin_cookie(headers: &HeaderMap, dev_api_key: &str) -> bool {
    let Some(value) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    value
        .split(';')
        .map(|s| s.trim())
        .any(|kv| kv == format!("fa_admin={dev_api_key}"))
}

async fn admin_page(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Html<String> {
    if !has_admin_cookie(&headers, &state.dev_api_key) {
        let body = r#"
<div class="card">
  <h2>Admin Login</h2>
  <p class="muted">Enter <code>DEV_API_KEY</code> from your environment to unlock admin-only controls.</p>
  <label>Password</label>
  <input id="pass" type="password" />
  <button id="login">Login</button>
  <pre id="status"></pre>
</div>
<script>
document.getElementById('login').addEventListener('click', async () => {
  const password = document.getElementById('pass').value;
  const s = document.getElementById('status');
  const r = await fetch('/admin/login', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({password})});
  const d = await r.json();
  if (!r.ok) { s.textContent = d.error || 'login failed'; return; }
  location.reload();
});
</script>
"#;
        return Html(page_shell("FAIL Academy Admin", body));
    }

    let body = r#"
<div class="row">
  <div class="card">
    <h2>Admin Controls</h2>
    <p class="muted">These operations are proxied to ollama-controls API.</p>
    <button id="refresh-tags">List Tags</button>
    <button id="pull-btn">Pull Model</button>
    <input id="pull-name" value="llama3.2:latest" />
    <button id="gen-btn">Run Generate</button>
    <input id="gen-model" value="llama3.2:latest" />
    <textarea id="gen-prompt" rows="4">Say: admin path works.</textarea>
  </div>
  <div class="card">
    <h3>Output</h3>
    <pre id="admin-out" style="white-space:pre-wrap"></pre>
  </div>
</div>
<script>
const out = document.getElementById('admin-out');
document.getElementById('refresh-tags').onclick = async () => {
  const r = await fetch('/admin/api/tags'); out.textContent = JSON.stringify(await r.json(), null, 2);
};
document.getElementById('pull-btn').onclick = async () => {
  const name = document.getElementById('pull-name').value.trim();
  const r = await fetch('/admin/api/pull', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({name})});
  out.textContent = JSON.stringify(await r.json(), null, 2);
};
document.getElementById('gen-btn').onclick = async () => {
  const model = document.getElementById('gen-model').value.trim();
  const prompt = document.getElementById('gen-prompt').value.trim();
  const r = await fetch('/admin/api/generate', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({model,prompt})});
  out.textContent = JSON.stringify(await r.json(), null, 2);
};
</script>
"#;
    Html(page_shell("FAIL Academy Admin", body))
}

async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AdminLoginBody>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if body.password != state.dev_api_key {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid password" }))));
    }
    let mut resp = Json(json!({"ok": true})).into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        format!("fa_admin={}; Path=/; HttpOnly; SameSite=Lax", state.dev_api_key)
            .parse()
            .expect("set-cookie"),
    );
    Ok(resp)
}

fn require_admin(headers: &HeaderMap, dev_api_key: &str) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if has_admin_cookie(headers, dev_api_key) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, Json(json!({ "error": "admin authentication required" }))))
    }
}

async fn admin_tags(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&headers, &state.dev_api_key)?;
    state
        .client
        .list_tags()
        .map(|v| Json(json!(v)))
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))))
}

async fn admin_pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PullBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&headers, &state.dev_api_key)?;
    state
        .client
        .pull_model(&body.name)
        .map(|v| Json(json!(v)))
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))))
}

async fn admin_generate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AdminGenerateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&headers, &state.dev_api_key)?;
    state
        .client
        .generate(&body.model, &body.prompt)
        .map(|v| Json(json!(v)))
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))))
}
