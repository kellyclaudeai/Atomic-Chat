use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const BRAVE_SEARCH_SERVICE_NAME: &str = "Atomic Chat";
const BRAVE_SEARCH_ACCOUNT_NAME: &str = "brave-search-api-key";
const BRAVE_SEARCH_ENDPOINT: &str = "https://api.search.brave.com/res/v1/llm/context";
const DEFAULT_CONTEXT_WINDOW: usize = 8_192;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraveSearchContextRequest {
    pub query: String,
    pub context_window: Option<usize>,
    pub is_incognito: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraveSearchContextPayload {
    pub context_message: String,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use keyring::{Entry, Error as KeyringError};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn keyring_entry() -> Result<Entry, String> {
    Entry::new(BRAVE_SEARCH_SERVICE_NAME, BRAVE_SEARCH_ACCOUNT_NAME).map_err(map_keyring_error)
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn map_keyring_error(error: KeyringError) -> String {
    match error {
        KeyringError::NoEntry => "No Brave Search API key is currently stored.".to_string(),
        other => format!("Failed to access Brave Search secure storage: {other}"),
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn read_brave_search_api_key() -> Result<String, String> {
    let entry = keyring_entry()?;
    match entry.get_password() {
        Ok(api_key) => Ok(api_key),
        Err(KeyringError::NoEntry) => {
            Err("Add a Brave Search API key in Settings to use Brave grounding.".to_string())
        }
        Err(error) => Err(map_keyring_error(error)),
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub fn get_brave_search_api_key() -> Result<String, String> {
    let entry = keyring_entry()?;
    match entry.get_password() {
        Ok(api_key) => Ok(api_key),
        Err(KeyringError::NoEntry) => Ok(String::new()),
        Err(error) => Err(map_keyring_error(error)),
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub fn get_brave_search_api_key() -> Result<String, String> {
    Err("Brave Search secure storage is only available on desktop.".to_string())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub fn set_brave_search_api_key(api_key: String) -> Result<(), String> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err("Brave Search API key cannot be empty.".to_string());
    }

    let entry = keyring_entry()?;
    entry.set_password(trimmed).map_err(map_keyring_error)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub fn set_brave_search_api_key(_api_key: String) -> Result<(), String> {
    Err("Brave Search secure storage is only available on desktop.".to_string())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub fn clear_brave_search_api_key() -> Result<(), String> {
    let entry = keyring_entry()?;
    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(map_keyring_error(error)),
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub fn clear_brave_search_api_key() -> Result<(), String> {
    Err("Brave Search secure storage is only available on desktop.".to_string())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn get_brave_search_context(
    request: BraveSearchContextRequest,
) -> Result<BraveSearchContextPayload, String> {
    let api_key = read_brave_search_api_key()?;
    let normalized_query = normalized_query(&request.query);
    if normalized_query.is_empty() {
        return Err("Enter a message before using Brave grounding.".to_string());
    }

    let context_window = request.context_window.unwrap_or(DEFAULT_CONTEXT_WINDOW);
    let full_request = BraveRequest {
        q: normalized_query.clone(),
        country: Some("us".to_string()),
        search_lang: Some("en".to_string()),
        count: Some(request_count(context_window)),
        freshness: freshness(&normalized_query),
        maximum_number_of_urls: Some(request_count(context_window)),
        maximum_number_of_tokens: Some(token_budget(context_window)),
        maximum_number_of_snippets: Some(18),
        maximum_number_of_tokens_per_url: Some(768),
        maximum_number_of_snippets_per_url: Some(2),
        context_threshold_mode: Some("balanced".to_string()),
    };

    let response = match search_response(&full_request, &api_key).await {
        Ok(response) => response,
        Err(message) if should_retry_with_minimal_parameters(&message) => {
            let fallback_request = BraveRequest {
                q: normalized_query.clone(),
                country: None,
                search_lang: None,
                count: None,
                freshness: freshness(&normalized_query),
                maximum_number_of_urls: None,
                maximum_number_of_tokens: None,
                maximum_number_of_snippets: None,
                maximum_number_of_tokens_per_url: None,
                maximum_number_of_snippets_per_url: None,
                context_threshold_mode: None,
            };
            search_response(&fallback_request, &api_key).await?
        }
        Err(message) => return Err(message),
    };

    Ok(BraveSearchContextPayload {
        context_message: make_context_message(&response, &normalized_query, request.is_incognito),
    })
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub async fn get_brave_search_context(
    _request: BraveSearchContextRequest,
) -> Result<BraveSearchContextPayload, String> {
    Err("Brave Search grounding is only available on desktop.".to_string())
}

async fn search_response(
    request_body: &BraveRequest,
    api_key: &str,
) -> Result<BraveResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Failed to initialize Brave Search client: {error}"))?;

    let response = client
        .post(BRAVE_SEARCH_ENDPOINT)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("X-Subscription-Token", api_key)
        .json(request_body)
        .send()
        .await
        .map_err(|error| format!("Brave Search request failed: {error}"))?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read Brave Search response: {error}"))?;

    if !status.is_success() {
        let message = server_error_message(&body)
            .unwrap_or_else(|| "Brave Search request failed.".to_string());
        return Err(message);
    }

    serde_json::from_slice::<BraveResponse>(&body)
        .map_err(|error| format!("Brave Search returned an unexpected response: {error}"))
}

fn request_count(context_window: usize) -> usize {
    match context_window {
        0..=8_191 => 4,
        8_192..=16_383 => 5,
        16_384..=32_767 => 6,
        _ => 8,
    }
}

fn token_budget(context_window: usize) -> usize {
    (context_window / 4).clamp(2_048, 6_144)
}

fn freshness(query: &str) -> Option<String> {
    let normalized = query.to_lowercase();

    if normalized.contains("today")
        || normalized.contains("breaking")
        || normalized.contains("latest")
        || normalized.contains("right now")
        || normalized.contains("current news")
    {
        return Some("pd".to_string());
    }

    if normalized.contains("this week") || normalized.contains("recent") {
        return Some("pw".to_string());
    }

    if normalized.contains("this month") {
        return Some("pm".to_string());
    }

    None
}

fn normalized_query(query: &str) -> String {
    let collapsed = query
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if collapsed.is_empty() {
        return query.trim().to_string();
    }

    let limited_words = collapsed
        .split_whitespace()
        .take(50)
        .collect::<Vec<_>>()
        .join(" ");

    let trimmed = limited_words.trim();
    if trimmed.chars().count() <= 400 {
        return trimmed.to_string();
    }

    let prefix = trimmed.chars().take(400).collect::<String>();
    if let Some(last_whitespace_index) = prefix.char_indices().rfind(|(_, ch)| ch.is_whitespace()) {
        let shortened = prefix[..last_whitespace_index.0].trim();
        if !shortened.is_empty() {
            return shortened.to_string();
        }
    }

    prefix.trim().to_string()
}

fn should_retry_with_minimal_parameters(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("validate parameters")
        || normalized.contains("unable to validate parameters")
        || normalized.contains("invalid parameter")
}

fn server_error_message(data: &[u8]) -> Option<String> {
    if let Ok(envelope) = serde_json::from_slice::<ErrorEnvelope>(data) {
        if let Some(error) = envelope.error {
            if let Some(message) = trim_or_none(error.message) {
                return Some(message);
            }
            if let Some(detail) = trim_or_none(error.detail) {
                return Some(detail);
            }
        }
    }

    let message = String::from_utf8_lossy(data).trim().to_string();
    if message.is_empty() {
        None
    } else {
        Some(message)
    }
}

fn make_context_message(response: &BraveResponse, query: &str, is_incognito: bool) -> String {
    let sources = grounded_sources(response);
    let mut intro_lines = vec![
        "Live Brave Search grounding is enabled for the latest user message.".to_string(),
        "Use the web context below when it is relevant. Prefer this grounding over stale model memory for time-sensitive claims.".to_string(),
        "Cite source hostnames or URLs when you rely on this web grounding.".to_string(),
    ];

    if is_incognito {
        intro_lines.push(
            "This request came from an incognito chat, so treat the web grounding as ephemeral session context only."
                .to_string(),
        );
    }

    intro_lines.push(format!("Search query: {query}"));
    let intro = intro_lines.join("\n");

    if sources.is_empty() {
        return format!(
            "{intro}\n\nNo relevant Brave Search grounding was returned for this query."
        );
    }

    let source_lines = sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let mut lines = vec![
                format!("{}. {}", index + 1, source.title),
                format!("URL: {}", source.url),
            ];

            if let Some(hostname) = &source.hostname {
                lines.push(format!("Host: {hostname}"));
            }

            if let Some(age) = &source.age {
                lines.push(format!("Age: {age}"));
            }

            if !source.snippets.is_empty() {
                lines.push("Snippets:".to_string());
                lines.extend(source.snippets.iter().map(|snippet| format!("- {snippet}")));
            }

            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("{intro}\n\nWeb Sources:\n{source_lines}")
}

fn grounded_sources(response: &BraveResponse) -> Vec<GroundedSource> {
    let mut ordered = Vec::new();
    let mut resolved: HashMap<String, GroundedSource> = HashMap::new();

    let mut insert = |result: &GroundingResult| {
        let normalized_url = result.url.trim().to_string();
        if normalized_url.is_empty() {
            return;
        }

        let metadata = response
            .sources
            .as_ref()
            .and_then(|sources| sources.get(&normalized_url));
        let snippets = result
            .snippets
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|snippet| normalize_snippet(&snippet))
            .filter(|snippet| !snippet.is_empty())
            .take(2)
            .collect::<Vec<_>>();

        if !resolved.contains_key(&normalized_url) {
            ordered.push(normalized_url.clone());
            resolved.insert(
                normalized_url.clone(),
                GroundedSource {
                    url: normalized_url.clone(),
                    title: trim_or_none(result.name.clone())
                        .or_else(|| trim_or_none(result.title.clone()))
                        .or_else(|| metadata.and_then(|value| trim_or_none(value.title.clone())))
                        .unwrap_or(normalized_url),
                    hostname: metadata.and_then(|value| trim_or_none(value.hostname.clone())),
                    age: metadata.and_then(|value| {
                        value
                            .age
                            .as_ref()
                            .and_then(|ages| ages.first().cloned())
                            .and_then(|age| trim_or_none(Some(age)))
                    }),
                    snippets,
                },
            );
            return;
        }

        if let Some(existing) = resolved.get_mut(&normalized_url) {
            if existing.snippets.len() < 2 {
                for snippet in snippets {
                    if !existing.snippets.contains(&snippet) {
                        existing.snippets.push(snippet);
                    }
                    if existing.snippets.len() == 2 {
                        break;
                    }
                }
            }
        }
    };

    if let Some(grounding) = response.grounding.as_ref() {
        if let Some(poi) = grounding.poi.as_ref() {
            insert(poi);
        }
        if let Some(map) = grounding.map.as_ref() {
            for result in map {
                insert(result);
            }
        }
        if let Some(generic) = grounding.generic.as_ref() {
            for result in generic {
                insert(result);
            }
        }
    }

    ordered
        .into_iter()
        .filter_map(|url| resolved.remove(&url))
        .take(6)
        .collect()
}

fn normalize_snippet(snippet: &str) -> String {
    let collapsed = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 280 {
        let mut shortened = collapsed.chars().take(277).collect::<String>();
        shortened.push_str("...");
        return shortened;
    }
    collapsed
}

fn trim_or_none(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct BraveRequest {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    freshness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_number_of_urls: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_number_of_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_number_of_snippets: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_number_of_tokens_per_url: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_number_of_snippets_per_url: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_threshold_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BraveResponse {
    grounding: Option<Grounding>,
    sources: Option<HashMap<String, SourceMetadata>>,
}

#[derive(Debug, Deserialize)]
struct Grounding {
    generic: Option<Vec<GroundingResult>>,
    poi: Option<GroundingResult>,
    map: Option<Vec<GroundingResult>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GroundingResult {
    name: Option<String>,
    url: String,
    title: Option<String>,
    snippets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SourceMetadata {
    title: Option<String>,
    hostname: Option<String>,
    age: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct ErrorPayload {
    message: Option<String>,
    detail: Option<String>,
}

#[derive(Debug)]
struct GroundedSource {
    url: String,
    title: String,
    hostname: Option<String>,
    age: Option<String>,
    snippets: Vec<String>,
}
