use crate::db::Db;
use serde_json::Value;

/// AI Provider configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub adapter_type: String,
    pub base_url: String,
    pub api_key_ref: Option<String>,
    pub default_model: String,
    pub available_models: Vec<String>,
    pub is_local: bool,
    pub is_default: bool,
    pub enabled: bool,
    pub config: Value,
}

/// Create or update an AI provider.
pub fn upsert_provider(provider: &AiProvider, db: &Db) -> Result<(), String> {
    if !matches!(
        provider.adapter_type.as_str(),
        "ollama" | "openai_compatible" | "deepseek" | "openai" | "anthropic"
    ) {
        return Err(format!(
            "Unsupported adapter type: {}",
            provider.adapter_type
        ));
    }
    if provider.name.trim().is_empty() {
        return Err("Provider name cannot be empty".into());
    }
    if provider.default_model.trim().is_empty() {
        return Err("Default model cannot be empty".into());
    }
    validate_base_url(&provider.base_url)?;
    if let Some(api_key_ref) = &provider.api_key_ref {
        if looks_like_secret(api_key_ref) {
            return Err("apiKeyRef must be an environment variable name, not a raw secret".into());
        }
        if !is_valid_env_var_name(api_key_ref) {
            return Err("apiKeyRef must be a valid environment variable name".into());
        }
    }
    let serialized_config = serde_json::to_string(&provider.config).map_err(|e| e.to_string())?;
    if !scan_for_secrets(&serialized_config).is_empty() {
        return Err("Provider config must not contain raw secrets; use apiKeyRef instead".into());
    }

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    if provider.is_default {
        tx.execute(
            "UPDATE ai_provider SET is_default = 0 WHERE id <> ?1",
            rusqlite::params![provider.id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT INTO ai_provider (id, name, adapter_type, base_url, api_key_ref, default_model, available_models, is_local, is_default, enabled, config, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'), datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             adapter_type = excluded.adapter_type,
             base_url = excluded.base_url,
             api_key_ref = excluded.api_key_ref,
             default_model = excluded.default_model,
             available_models = excluded.available_models,
             is_local = excluded.is_local,
             is_default = excluded.is_default,
             enabled = excluded.enabled,
             config = excluded.config,
             updated_at = datetime('now')",
        rusqlite::params![
            provider.id,
            provider.name,
            provider.adapter_type,
            provider.base_url,
            provider.api_key_ref,
            provider.default_model,
            serde_json::to_string(&provider.available_models).unwrap_or_default(),
            provider.is_local as i32,
            provider.is_default as i32,
            provider.enabled as i32,
            serialized_config,
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// List all AI providers.
pub fn list_providers(db: &Db) -> Result<Vec<AiProvider>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, adapter_type, base_url, api_key_ref, default_model, available_models, is_local, is_default, enabled, config FROM ai_provider ORDER BY name")
        .map_err(|e| e.to_string())?;

    let providers = stmt
        .query_map([], |row| {
            let models_str: String = row.get(6)?;
            let config_str: String = row.get(10)?;
            Ok(AiProvider {
                id: row.get(0)?,
                name: row.get(1)?,
                adapter_type: row.get(2)?,
                base_url: row.get(3)?,
                api_key_ref: row.get(4)?,
                default_model: row.get(5)?,
                available_models: serde_json::from_str(&models_str).unwrap_or_default(),
                is_local: row.get::<_, i32>(7)? != 0,
                is_default: row.get::<_, i32>(8)? != 0,
                enabled: row.get::<_, i32>(9)? != 0,
                config: serde_json::from_str(&config_str)
                    .unwrap_or(Value::Object(Default::default())),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(providers)
}

/// Delete an AI provider.
pub fn delete_provider(id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM ai_provider WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Detect local AI providers (e.g. Ollama).
pub fn detect_local_providers() -> Vec<AiProvider> {
    let mut providers = Vec::new();

    // Check for Ollama
    let ollama_running = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .and_then(|client| client.get("http://localhost:11434/api/tags").send())
        .map(|response| response.status().is_success())
        .unwrap_or(false);

    if ollama_running {
        let models = fetch_ollama_models();
        providers.push(AiProvider {
            id: "ollama-local".into(),
            name: "Ollama (Local)".into(),
            adapter_type: "ollama".into(),
            base_url: "http://localhost:11434".into(),
            api_key_ref: None,
            default_model: models.first().cloned().unwrap_or_else(|| "llama3".into()),
            available_models: models,
            is_local: true,
            is_default: false,
            enabled: true,
            config: serde_json::json!({}),
        });
    }

    providers
}

fn fetch_ollama_models() -> Vec<String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()
        .and_then(|client| client.get("http://localhost:11434/api/tags").send().ok())
        .and_then(|response| response.json::<Value>().ok())
        .and_then(|v| {
            v.get("models")?
                .as_array()?
                .iter()
                .filter_map(|m| m.get("name")?.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .into()
        })
        .unwrap_or_default()
}

/// ContextPack: assembled context for an AI call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPack {
    pub purpose: String,
    pub system_prompt: Option<String>,
    pub sections: Vec<ContextSection>,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSection {
    pub label: String,
    pub source: String,
    pub content: String,
    pub tokens_estimate: usize,
}

impl ContextPack {
    pub fn new(purpose: &str, max_tokens: usize) -> Self {
        Self {
            purpose: purpose.to_string(),
            system_prompt: None,
            sections: Vec::new(),
            max_tokens,
        }
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    pub fn add_section(mut self, label: &str, source: &str, content: &str) -> Self {
        let tokens_estimate = content.split_whitespace().count(); // rough: 1 token ≈ 1 word
        self.sections.push(ContextSection {
            label: label.to_string(),
            source: source.to_string(),
            content: content.to_string(),
            tokens_estimate,
        });
        self
    }

    pub fn build_body(&self) -> String {
        let mut parts = Vec::new();
        let used = if let Some(ref system) = self.system_prompt {
            system.split_whitespace().count()
        } else {
            0
        };
        let remaining = self.max_tokens.saturating_sub(used);

        let mut budget = remaining;
        for section in &self.sections {
            if section.tokens_estimate <= budget {
                parts.push(format!(
                    "## {}\n[Source: {}]\n{}",
                    section.label, section.source, section.content
                ));
                budget -= section.tokens_estimate;
            } else if budget > 100 {
                // Truncate to fit
                let words: Vec<&str> = section
                    .content
                    .split_whitespace()
                    .take(budget - 20)
                    .collect();
                parts.push(format!(
                    "## {} (truncated)\n[Source: {}]\n{}...",
                    section.label,
                    section.source,
                    words.join(" ")
                ));
                budget = 0;
            }
        }

        parts.join("\n\n")
    }

    /// Build the final prompt, respecting token budget.
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref system) = self.system_prompt {
            parts.push(system.clone());
        }

        parts.push(self.build_body());
        parts.join("\n\n")
    }
}
use std::sync::OnceLock;

static SECRET_REGEXES: OnceLock<Vec<(regex::Regex, &'static str)>> = OnceLock::new();

fn get_secret_regexes() -> &'static Vec<(regex::Regex, &'static str)> {
    SECRET_REGEXES.get_or_init(|| {
        let patterns = [
            (
                r#"(?i)(api[_-]?key|apikey|secret|token|password|passwd|auth)\s*[:=]\s*['"]?[A-Za-z0-9+/=_-]{16,}"#,
                "API key/secret",
            ),
            (r"ghp_[A-Za-z0-9]{36}", "GitHub PAT"),
            (r"gho_[A-Za-z0-9]{36}", "GitHub OAuth"),
            (r"sk-[A-Za-z0-9]{48}", "OpenAI API key"),
            (
                r"-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----",
                "Private key",
            ),
            (r"AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
            (
                r"eyJ[A-Za-z0-9_-]*\.eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*",
                "JWT token",
            ),
        ];

        patterns
            .iter()
            .filter_map(|(pattern, label)| {
                regex::Regex::new(pattern).ok().map(|re| (re, *label))
            })
            .collect()
    })
}

static REDACT_REGEXES: OnceLock<Vec<regex::Regex>> = OnceLock::new();

fn get_redact_regexes() -> &'static Vec<regex::Regex> {
    REDACT_REGEXES.get_or_init(|| {
        let patterns = [
            r#"(?i)(api[_-]?key|apikey|secret|token|password|passwd|auth)\s*[:=]\s*['"]?[A-Za-z0-9+/=_-]{16,}"#,
            r"ghp_[A-Za-z0-9]{36}",
            r"gho_[A-Za-z0-9]{36}",
            r"sk-[A-Za-z0-9]{48}",
            r"AKIA[0-9A-Z]{16}",
            r"eyJ[A-Za-z0-9_-]*\.eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*",
            r"(?s)-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----.*?-----END (?:RSA |EC |DSA )?PRIVATE KEY-----",
        ];

        patterns
            .iter()
            .filter_map(|pattern| regex::Regex::new(pattern).ok())
            .collect()
    })
}

/// Scan content for potential secrets before sending to AI.
pub fn scan_for_secrets(content: &str) -> Vec<SecretMatch> {
    let mut matches = Vec::new();
    for (re, label) in get_secret_regexes() {
        for mat in re.find_iter(content) {
            matches.push(SecretMatch {
                label: label.to_string(),
                position: mat.start(),
                preview: "[REDACTED]".into(),
            });
        }
    }

    matches
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretMatch {
    pub label: String,
    pub position: usize,
    pub preview: String,
}

/// Redact secrets from content before sending to AI.
pub fn redact_secrets(content: &str) -> String {
    let mut result = content.to_string();
    for re in get_redact_regexes() {
        result = re.replace_all(&result, "[REDACTED]").to_string();
    }

    result
}

/// Call an AI provider.
pub async fn call_ai(
    provider: &AiProvider,
    prompt: &str,
    system_prompt: Option<&str>,
    model: Option<&str>,
) -> Result<AiResponse, String> {
    validate_base_url(&provider.base_url)?;
    let model = model.unwrap_or(&provider.default_model);
    if model.trim().is_empty() {
        return Err("AI model cannot be empty".into());
    }

    match provider.adapter_type.as_str() {
        "ollama" => call_ollama(provider, system_prompt, model, prompt).await,
        "openai_compatible" | "deepseek" | "openai" => {
            call_openai_compatible(provider, system_prompt, model, prompt).await
        }
        "anthropic" => call_anthropic(provider, system_prompt, model, prompt).await,
        _ => Err(format!(
            "Unsupported adapter type: {}",
            provider.adapter_type
        )),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProbe {
    pub reachable: bool,
    pub message: String,
    pub latency_ms: u64,
    pub models: Vec<String>,
}

fn get_openai_url(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{}/{}", base, suffix)
    } else {
        format!("{}/v1/{}", base, suffix)
    }
}

fn get_anthropic_url(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{}/{}", base, suffix)
    } else if base.ends_with("/v1/messages") {
        if suffix == "messages" {
            base.to_string()
        } else {
            format!("{}/{}", base.trim_end_matches("/messages"), suffix)
        }
    } else {
        format!("{}/v1/{}", base, suffix)
    }
}

pub async fn probe_provider(provider: &AiProvider) -> ProviderProbe {
    let started = std::time::Instant::now();
    let result = async {
        validate_base_url(&provider.base_url)?;
        match provider.adapter_type.as_str() {
            "ollama" => {
                let response = http_client()?
                    .get(format!(
                        "{}/api/tags",
                        provider.base_url.trim_end_matches('/')
                    ))
                    .send()
                    .await
                    .map_err(|error| format!("Cannot reach Ollama: {}", error))?
                    .error_for_status()
                    .map_err(|error| format!("Ollama health check failed: {}", error))?
                    .json::<Value>()
                    .await
                    .map_err(|error| format!("Invalid Ollama model response: {}", error))?;
                Ok(response
                    .get("models")
                    .and_then(|value| value.as_array())
                    .map(|models| {
                        models
                            .iter()
                            .filter_map(|model| {
                                model
                                    .get("name")
                                    .and_then(|value| value.as_str())
                                    .map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default())
            }
            "openai_compatible" | "deepseek" | "openai" => {
                let url = get_openai_url(&provider.base_url, "models");
                let mut request = http_client()?.get(url);
                if let Some(key_ref) = &provider.api_key_ref {
                    if let Ok(key) = std::env::var(key_ref) {
                        request = request.bearer_auth(key);
                    }
                }
                let response = request
                    .send()
                    .await
                    .map_err(|error| format!("Cannot reach provider: {}", error))?
                    .error_for_status()
                    .map_err(|error| format!("Provider health check failed: {}", error))?
                    .json::<Value>()
                    .await
                    .map_err(|error| format!("Invalid provider model response: {}", error))?;
                Ok(response
                    .get("data")
                    .and_then(|value| value.as_array())
                    .map(|models| {
                        models
                            .iter()
                            .filter_map(|model| {
                                model
                                    .get("id")
                                    .and_then(|value| value.as_str())
                                    .map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default())
            }
            "anthropic" => {
                let response = http_client()?
                    .get(format!("{}", provider.base_url.trim_end_matches('/')))
                    .send()
                    .await;
                match response {
                    Ok(_) => Ok(vec![
                        "claude-3-5-sonnet-20241022".to_string(),
                        "claude-3-5-haiku-20241022".to_string(),
                        "claude-3-opus-20240229".to_string(),
                    ]),
                    Err(e) => Err(format!("Cannot reach Anthropic: {}", e)),
                }
            }
            _ => Err(format!(
                "Unsupported adapter type: {}",
                provider.adapter_type
            )),
        }
    }
    .await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(models) => ProviderProbe {
            reachable: true,
            message: format!("Provider reachable; {} model(s) reported", models.len()),
            latency_ms,
            models,
        },
        Err(message) => ProviderProbe {
            reachable: false,
            message,
            latency_ms,
            models: Vec::new(),
        },
    }
}

async fn call_ollama(
    provider: &AiProvider,
    system_prompt: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<AiResponse, String> {
    let mut payload = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
    });
    if let Some(system) = system_prompt {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("system".to_string(), serde_json::json!(system));
        }
    }
    // Merge custom provider.config parameters as options
    if provider.config.is_object() {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("options".to_string(), provider.config.clone());
        }
    }

    let response = http_client()?
        .post(format!(
            "{}/api/generate",
            provider.base_url.trim_end_matches('/')
        ))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call Ollama: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Ollama call failed: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Invalid Ollama response: {}", e))?;

    Ok(AiResponse {
        content: response
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        model: model.to_string(),
        tokens_in: response
            .get("prompt_eval_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        tokens_out: response
            .get("eval_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        finish_reason: None,
    })
}

async fn call_openai_compatible(
    provider: &AiProvider,
    system_prompt: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<AiResponse, String> {
    let api_key = match &provider.api_key_ref {
        Some(key_ref) => std::env::var(key_ref).map_err(|_| {
            format!(
                "Environment variable '{}' is not set for this provider",
                key_ref
            )
        })?,
        None => {
            return Err(
                "No API key environment variable configured for OpenAI-compatible provider".into(),
            )
        }
    };

    let mut messages = Vec::new();
    if let Some(system) = system_prompt {
        messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    messages.push(serde_json::json!({
        "role": "user",
        "content": prompt
    }));

    let mut payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": 4096,
    });

    // Merge custom provider.config parameters
    if let Some(config_obj) = provider.config.as_object() {
        if let Some(payload_obj) = payload.as_object_mut() {
            for (k, v) in config_obj {
                if k != "model" && k != "messages" {
                    payload_obj.insert(k.clone(), v.clone());
                }
            }
        }
    }

    let url = get_openai_url(&provider.base_url, "chat/completions");
    let response = http_client()?
        .post(url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call OpenAI-compatible API: {}", e))?
        .error_for_status()
        .map_err(|e| format!("API call failed: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Invalid API response: {}", e))?;

    let choice = response
        .get("choices")
        .and_then(|c| c.get(0))
        .ok_or("No choices in response")?;
    let content = choice
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let usage = response.get("usage");

    Ok(AiResponse {
        content,
        model: model.to_string(),
        tokens_in: usage
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize,
        tokens_out: usage
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize,
        finish_reason: choice
            .get("finish_reason")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string()),
    })
}

async fn call_anthropic(
    provider: &AiProvider,
    system_prompt: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<AiResponse, String> {
    let api_key = match &provider.api_key_ref {
        Some(key_ref) => std::env::var(key_ref).map_err(|_| {
            format!(
                "Environment variable '{}' is not set for this provider",
                key_ref
            )
        })?,
        None => return Err("No API key environment variable configured for Anthropic provider".into()),
    };

    let mut payload = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    if let Some(system) = system_prompt {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("system".to_string(), serde_json::json!(system));
        }
    }

    // Merge custom provider.config parameters
    if let Some(config_obj) = provider.config.as_object() {
        if let Some(payload_obj) = payload.as_object_mut() {
            for (k, v) in config_obj {
                if k != "model" && k != "messages" && k != "system" {
                    payload_obj.insert(k.clone(), v.clone());
                }
            }
        }
    }

    let url = get_anthropic_url(&provider.base_url, "messages");
    let request = http_client()?
        .post(url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .bearer_auth(&api_key); // Maximize compatibility with gateways (OneAPI/LiteLLM/OpenRouter)

    let response = request
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call Anthropic API: {}", e))?
        .error_for_status()
        .map_err(|e| format!("API call failed: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Invalid API response: {}", e))?;

    let content = response
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .ok_or("No content text in response")?
        .to_string();

    let usage = response.get("usage");

    Ok(AiResponse {
        content,
        model: model.to_string(),
        tokens_in: usage
            .and_then(|u| u.get("input_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize,
        tokens_out: usage
            .and_then(|u| u.get("output_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize,
        finish_reason: response
            .get("stop_reason")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string()),
    })
}

fn looks_like_secret(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("sk-")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("gho_")
        || trimmed.starts_with("AKIA")
        || trimmed.contains("-----BEGIN ")
        || trimmed.len() > 80
        || trimmed.chars().any(char::is_whitespace)
}

fn is_valid_env_var_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('_') | Some('A'..='Z') | Some('a'..='z'))
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn validate_base_url(value: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(value.trim()).map_err(|_| "Invalid provider base URL")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Provider base URL must be an absolute HTTP or HTTPS URL".into());
    }
    if url.username() != "" || url.password().is_some() {
        return Err("Provider base URL must not contain embedded credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Provider base URL must not contain a query or fragment".into());
    }
    Ok(())
}

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> Result<&'static reqwest::Client, String> {
    Ok(CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .expect("Failed to build global HTTP client")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_provider_base_urls() {
        assert!(validate_base_url("http://localhost:11434").is_ok());
        assert!(validate_base_url("https://api.example.com/v1").is_ok());
        assert!(validate_base_url("file:///tmp/provider").is_err());
        assert!(validate_base_url("https://user:secret@example.com").is_err());
        assert!(validate_base_url("https://example.com?token=secret").is_err());
    }

    #[test]
    fn validates_environment_variable_names() {
        assert!(is_valid_env_var_name("OPENAI_API_KEY"));
        assert!(is_valid_env_var_name("_LOCAL_KEY_2"));
        assert!(!is_valid_env_var_name("OPENAI-API-KEY"));
        assert!(!is_valid_env_var_name("1OPENAI_KEY"));
        assert!(!is_valid_env_var_name("KEY=value"));
    }

    #[test]
    fn secret_matches_never_echo_secret_material() {
        let matches = scan_for_secrets("token=abcdefghijklmnop123456");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].preview, "[REDACTED]");
    }
}
