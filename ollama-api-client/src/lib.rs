use serde::{Deserialize, Serialize};

/// Simple client for the `ollama-controls-api` HTTP gateway.
#[derive(Debug, Clone)]
pub struct OllamaControlsApiClient {
    base_url: String,
    api_key: Option<String>,
    agent: ureq::Agent,
}

impl OllamaControlsApiClient {
    /// Create a new client with an explicit base URL (e.g. `http://127.0.0.1:3000`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: trim_slash(&base_url.into()).to_string(),
            api_key: None,
            agent: ureq::Agent::new(),
        }
    }

    /// Create a client from common env vars.
    ///
    /// - `OLLAMA_CONTROLS_API_URL` (default: `http://127.0.0.1:3000`)
    /// - `OLLAMA_CONTROLS_API_KEY` (optional bearer value)
    /// - `DEV_API_KEY` (optional fallback)
    pub fn from_env() -> Self {
        let base =
            std::env::var("OLLAMA_CONTROLS_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        let key = std::env::var("OLLAMA_CONTROLS_API_KEY")
            .ok()
            .or_else(|| std::env::var("DEV_API_KEY").ok())
            .filter(|s| !s.trim().is_empty());
        Self::new(base).with_api_key_opt(key)
    }

    /// Set bearer API key for authenticated endpoints.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_api_key_opt(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
        self
    }

    pub fn health(&self) -> Result<HealthResponse, String> {
        self.get_json("/health")
    }

    pub fn list_tags(&self) -> Result<TagsResponse, String> {
        self.get_json("/api/tags")
    }

    pub fn list_ps(&self) -> Result<PsResponse, String> {
        self.get_json("/api/ps")
    }

    pub fn show_model(&self, name: &str) -> Result<ShowResponse, String> {
        let path = format!("/api/show?name={}", urlencoding::encode(name));
        self.get_json(&path)
    }

    pub fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse, String> {
        self.post_json(
            "/api/generate",
            &GenerateRequest {
                model: model.to_string(),
                prompt: prompt.to_string(),
            },
        )
    }

    pub fn pull_model(&self, name: &str) -> Result<PullResponse, String> {
        self.post_json(
            "/api/pull",
            &PullRequest {
                name: name.to_string(),
            },
        )
    }

    pub fn delete_model(&self, name: &str) -> Result<(), String> {
        let path = format!("/api/delete?name={}", urlencoding::encode(name));
        self.delete_no_content(&path)
    }

    pub fn copy_model(&self, source: &str, destination: &str) -> Result<(), String> {
        self.post_no_content(
            "/api/copy",
            &CopyRequest {
                source: source.to_string(),
                destination: destination.to_string(),
            },
        )
    }

    pub fn create_model(&self, name: &str, modelfile: &str) -> Result<CreateResponse, String> {
        self.post_json(
            "/api/create",
            &CreateRequest {
                name: name.to_string(),
                modelfile: modelfile.to_string(),
            },
        )
    }

    pub fn create_from_base(&self, new_name: &str, from_model: &str) -> Result<CreateResponse, String> {
        self.post_json(
            "/api/create-from-base",
            &CreateFromBaseRequest {
                new_name: new_name.to_string(),
                from_model: from_model.to_string(),
            },
        )
    }

    pub fn unload_model(&self, model: &str) -> Result<(), String> {
        self.post_no_content(
            "/api/unload",
            &UnloadRequest {
                model: model.to_string(),
            },
        )
    }

    pub fn update_all(&self) -> Result<serde_json::Value, String> {
        self.post_json("/api/update-all", &serde_json::json!({}))
    }

    pub fn remove_except(&self, keep: &[String]) -> Result<serde_json::Value, String> {
        self.post_json(
            "/api/remove-except",
            &RemoveExceptRequest {
                keep: keep.to_vec(),
            },
        )
    }

    pub fn inspect_raw(&self, name: &str) -> Result<String, String> {
        let path = format!("/api/inspect/raw?name={}", urlencoding::encode(name));
        self.get_text(&path)
    }

    pub fn inspect_details(&self, name: &str) -> Result<serde_json::Value, String> {
        let path = format!("/api/inspect/details?name={}", urlencoding::encode(name));
        self.get_json(&path)
    }

    pub fn service_start(&self) -> Result<serde_json::Value, String> {
        self.post_json("/api/service/start", &serde_json::json!({}))
    }

    pub fn service_stop(&self) -> Result<(), String> {
        self.post_no_content("/api/service/stop", &serde_json::json!({}))
    }

    pub fn list_local_models(&self) -> Result<Vec<String>, String> {
        self.get_json("/api/local/models")
    }

    pub fn get_models_path(&self) -> Result<ModelsPathInfo, String> {
        self.get_json("/api/settings/models-path")
    }

    pub fn set_models_path(&self, path: &str) -> Result<ModelsPathInfo, String> {
        self.post_json(
            "/api/settings/models-path",
            &SetModelsPathRequest {
                path: path.to_string(),
            },
        )
    }

    fn get_json<T>(&self, path: &str) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        let req = self.auth_get(path);
        let resp = req.call().map_err(|e| e.to_string())?;
        Self::decode_json(resp)
    }

    fn post_json<Req, Resp>(&self, path: &str, body: &Req) -> Result<Resp, String>
    where
        Req: Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let req = self.auth_post(path);
        let resp = req.send_json(body).map_err(|e| e.to_string())?;
        Self::decode_json(resp)
    }

    fn post_no_content<Req>(&self, path: &str, body: &Req) -> Result<(), String>
    where
        Req: Serialize,
    {
        let req = self.auth_post(path);
        req.send_json(body).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete_no_content(&self, path: &str) -> Result<(), String> {
        let req = self.auth_delete(path);
        req.call().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_text(&self, path: &str) -> Result<String, String> {
        let req = self.auth_get(path);
        let resp = req.call().map_err(|e| e.to_string())?;
        resp.into_string().map_err(|e| e.to_string())
    }

    fn auth_get(&self, path: &str) -> ureq::Request {
        let req = self.agent.get(&self.url(path));
        self.apply_auth(req)
    }

    fn auth_post(&self, path: &str) -> ureq::Request {
        let req = self.agent.post(&self.url(path));
        self.apply_auth(req)
    }

    fn auth_delete(&self, path: &str) -> ureq::Request {
        let req = self.agent.delete(&self.url(path));
        self.apply_auth(req)
    }

    fn apply_auth(&self, req: ureq::Request) -> ureq::Request {
        if let Some(ref key) = self.api_key {
            req.set("Authorization", &format!("Bearer {key}"))
        } else {
            req
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn decode_json<T>(resp: ureq::Response) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        resp.into_json().map_err(|e| e.to_string())
    }
}

fn trim_slash(s: &str) -> &str {
    s.trim_end_matches('/')
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TagsResponse {
    pub models: Vec<ListedModel>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListedModel {
    pub name: String,
    pub model: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: Option<ModelTagDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelTagDetails {
    #[serde(default)]
    pub parent_model: String,
    pub format: Option<String>,
    pub family: Option<String>,
    pub families: Option<Vec<String>>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PsResponse {
    pub models: Vec<RunningModel>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunningModel {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub digest: String,
    pub details: Option<ModelTagDetails>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub size_vram: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShowResponse {
    pub license: Option<String>,
    pub modelfile: Option<String>,
    pub parameters: Option<String>,
    pub template: Option<String>,
    pub details: Option<serde_json::Value>,
    pub model_info: Option<serde_json::Value>,
    pub tensors: Option<serde_json::Value>,
    pub capabilities: Option<Vec<String>>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateResponse {
    pub model: String,
    #[serde(default)]
    pub created_at: Option<String>,
    pub response: String,
    pub done: bool,
    #[serde(default)]
    pub context: Option<Vec<i64>>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
    #[serde(default)]
    pub prompt_eval_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u64>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullResponse {
    pub lines: Vec<PullProgressLine>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullProgressLine {
    pub status: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub completed: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateResponse {
    pub lines: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsPathInfo {
    pub env: Option<String>,
    pub saved: Option<String>,
    pub effective: String,
    pub default: String,
    pub export_line: String,
}

#[derive(Debug, Clone, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Clone, Serialize)]
struct PullRequest {
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct CopyRequest {
    source: String,
    destination: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateRequest {
    name: String,
    modelfile: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateFromBaseRequest {
    new_name: String,
    #[serde(rename = "from")]
    from_model: String,
}

#[derive(Debug, Clone, Serialize)]
struct UnloadRequest {
    model: String,
}

#[derive(Debug, Clone, Serialize)]
struct RemoveExceptRequest {
    keep: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SetModelsPathRequest {
    path: String,
}
