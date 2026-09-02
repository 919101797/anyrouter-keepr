use std::collections::HashSet;
use std::time::Duration;

use chrono::Local;
use reqwest::{Client, Url};
use serde_json::Value;

use crate::core::claude_runtime_config::ClaudeCredential;
use crate::core::types::{UpstreamModel, UpstreamModelCatalog};

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const REQUEST_TIMEOUT_SECONDS: u64 = 8;

pub async fn fetch_upstream_models(
    upstream_url: &str,
    upstream_source: &str,
    credential: Option<&ClaudeCredential>,
) -> UpstreamModelCatalog {
    let checked_at = Local::now().to_rfc3339();
    let result = async {
        let models_url = models_url(upstream_url)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .map_err(|err| format!("创建上游请求失败：{err}"))?;
        let mut request = client
            .get(models_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header("anthropic-version", ANTHROPIC_API_VERSION);

        if let Some(credential) = credential {
            request = if is_api_key(&credential.kind) {
                request.header("x-api-key", &credential.value)
            } else {
                request.bearer_auth(&credential.value)
            };
        }

        let response = request
            .send()
            .await
            .map_err(|err| format!("无法连接当前上游：{err}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("上游模型接口返回 HTTP {status}"));
        }

        let body = response
            .bytes()
            .await
            .map_err(|err| format!("读取上游模型响应失败：{err}"))?;
        let value = serde_json::from_slice::<Value>(&body)
            .map_err(|err| format!("上游模型响应不是有效 JSON：{err}"))?;
        parse_models_response(&value)
    }
    .await;

    match result {
        Ok(models) => UpstreamModelCatalog {
            checked_at,
            upstream_url: upstream_url.to_string(),
            upstream_source: upstream_source.to_string(),
            models,
            error: None,
        },
        Err(error) => UpstreamModelCatalog {
            checked_at,
            upstream_url: upstream_url.to_string(),
            upstream_source: upstream_source.to_string(),
            models: Vec::new(),
            error: Some(error),
        },
    }
}

fn models_url(upstream_url: &str) -> Result<Url, String> {
    let mut url =
        Url::parse(upstream_url.trim()).map_err(|err| format!("当前上游地址无效：{err}"))?;
    let base_path = url.path().trim_end_matches('/');
    let path = if base_path.ends_with("/v1") {
        format!("{base_path}/models")
    } else {
        format!("{base_path}/v1/models")
    };
    url.set_path(&path);
    url.set_query(None);
    url.query_pairs_mut().append_pair("limit", "1000");
    Ok(url)
}

fn is_api_key(kind: &str) -> bool {
    let normalized = kind
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    normalized.contains("apikey")
}

fn parse_models_response(value: &Value) -> Result<Vec<UpstreamModel>, String> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "上游模型响应缺少 data 数组".to_string())?;
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for item in data {
        let Some(id) = item.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let display_name = item
            .get("display_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(id);
        models.push(UpstreamModel {
            id: id.to_string(),
            display_name: display_name.to_string(),
        });
    }

    if models.is_empty() {
        Err("上游未返回可用模型".to_string())
    } else {
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{is_api_key, models_url, parse_models_response};

    #[test]
    fn builds_models_url_from_root_or_v1_base() {
        assert_eq!(
            models_url("https://gateway.example/api").unwrap().as_str(),
            "https://gateway.example/api/v1/models?limit=1000"
        );
        assert_eq!(
            models_url("https://gateway.example/api/v1/")
                .unwrap()
                .as_str(),
            "https://gateway.example/api/v1/models?limit=1000"
        );
    }

    #[test]
    fn parses_and_deduplicates_dynamic_models() {
        let models = parse_models_response(&json!({
            "data": [
                { "id": "claude-future-6", "display_name": "Claude Future 6" },
                { "id": "gateway-alias" },
                { "id": "claude-future-6", "display_name": "duplicate" }
            ]
        }))
        .unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "claude-future-6");
        assert_eq!(models[0].display_name, "Claude Future 6");
        assert_eq!(models[1].display_name, "gateway-alias");
    }

    #[test]
    fn selects_auth_header_from_detected_credential_kind() {
        assert!(is_api_key("ANTHROPIC_API_KEY"));
        assert!(is_api_key("anthropicapikey"));
        assert!(!is_api_key("ANTHROPIC_AUTH_TOKEN"));
    }
}
