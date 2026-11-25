//! Which model service answers, plus the HTTP plumbing shared by the protocol clients.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::runtime::Handle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProviderKind {
    /// Google Gemini through its OpenAI-compatible endpoint (has a free tier).
    #[default]
    Gemini,
    /// Anthropic Claude (native Messages API, prompt caching).
    Claude,
    /// A model running on this PC (Ollama, LM Studio, llama.cpp server): free, no key.
    Local,
    /// Any other OpenAI-compatible service (OpenRouter, DeepSeek, Groq, …).
    Custom,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 4] = [ProviderKind::Gemini, ProviderKind::Claude, ProviderKind::Local, ProviderKind::Custom];

    pub fn label(self) -> &'static str {
        match self {
            ProviderKind::Gemini => "Google Gemini",
            ProviderKind::Claude => "Anthropic Claude",
            ProviderKind::Local => "Local model (Ollama, LM Studio)",
            ProviderKind::Custom => "Other OpenAI-compatible service",
        }
    }

    pub fn needs_key(self) -> bool {
        !matches!(self, ProviderKind::Local)
    }

    /// The commander sets this provider's address; Gemini and Claude always use their own.
    pub fn takes_address(self) -> bool {
        matches!(self, ProviderKind::Local | ProviderKind::Custom)
    }

    /// Where users get an API key.
    pub fn key_url(self) -> Option<&'static str> {
        match self {
            ProviderKind::Gemini => Some("https://aistudio.google.com/apikey"),
            ProviderKind::Claude => Some("https://platform.claude.com/settings/keys"),
            _ => None,
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            ProviderKind::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai/",
            ProviderKind::Claude => "https://api.anthropic.com/v1/",
            ProviderKind::Local => "http://localhost:11434/v1/",
            ProviderKind::Custom => "",
        }
    }

    /// Models tried in order when none is configured: cheap ones with good tool calling first.
    pub fn preferred_models(self) -> &'static [&'static str] {
        match self {
            ProviderKind::Gemini => &["gemini-3.5-flash-lite", "gemini-3.1-flash-lite", "gemini-2.5-flash-lite", "gemini-3.8-flash", "gemini-2.5-flash"],
            ProviderKind::Claude => &["claude-haiku-5-5", "claude-sonnet-5-5"],
            ProviderKind::Local | ProviderKind::Custom => &[],
        }
    }
}

/// The complete normalized API base, including its path. The provider is part of the credential identity.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Destination {
    kind: ProviderKind,
    base: String,
}

impl fmt::Debug for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Destination").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl Destination {
    pub fn new(kind: ProviderKind, address: &str) -> Result<Self, AiError> {
        let address = address.trim();
        let raw = if kind.takes_address() && !address.is_empty() { address } else { kind.default_base_url() };
        let mut base = reqwest::Url::parse(raw).map_err(|_| AiError::Destination)?;
        let loopback = base.host_str().is_some_and(|host| {
            host == "localhost" || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
        });
        if !(base.scheme() == "https" && base.has_host() || base.scheme() == "http" && loopback)
            || !base.username().is_empty() || base.password().is_some() || base.query().is_some() || base.fragment().is_some()
        {
            return Err(AiError::Destination);
        }
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        Ok(Self { kind, base: base.into() })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }
}

#[derive(Clone)]
struct Credential {
    destination: Destination,
    key: String,
}

/// An immutable model connection for one request. Keys are attached only to their exact destination.
#[derive(Clone, Default)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    pub base_url: String,
    credential: Option<Credential>,
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderConfig").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl ProviderConfig {
    pub fn new(kind: ProviderKind, base_url: String, model: String) -> Self {
        Self { kind, base_url, model, credential: None }
    }

    pub fn destination(&self) -> Result<Destination, AiError> {
        Destination::new(self.kind, &self.base_url)
    }

    pub fn with_credential(mut self, destination: Destination, key: String) -> Self {
        self.credential = Some(Credential { destination, key });
        self
    }

    fn key_for(&self, destination: &Destination) -> Option<&str> {
        self.credential.as_ref().filter(|c| c.destination == *destination).map(|c| c.key.trim()).filter(|key| !key.is_empty())
    }

    pub fn has_credential(&self) -> bool {
        self.destination().is_ok_and(|destination| self.key_for(&destination).is_some())
    }

    pub fn is_ready(&self) -> bool {
        self.destination().is_ok() && (!self.kind.needs_key() || self.has_credential())
    }
}

/// Model-only transport. Redirects are never followed. There is no fallback to a caller's ambient client.
#[derive(Clone)]
pub struct ProviderHttp {
    remote: Result<reqwest::Client, AiError>,
    loopback: Result<reqwest::Client, AiError>,
}

impl Default for ProviderHttp {
    fn default() -> Self {
        let client = |direct| {
            let builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).user_agent("EliteNexus");
            let builder = if direct { builder.no_proxy() } else { builder };
            builder.build().map_err(|_| AiError::Http("could not initialize the model connection".into()))
        };
        Self { remote: client(false), loopback: client(true) }
    }
}

impl ProviderHttp {
    pub(crate) fn request(&self, rt: &Handle, cfg: &ProviderConfig, path: &str, body: Option<&Value>, stop: Option<&AtomicBool>) -> Result<Value, AiError> {
        let destination = cfg.destination()?;
        // Only loopback is allowed to use HTTP. Keep cleartext loopback requests out of ambient proxies.
        let connection = if destination.base_url().starts_with("http://") { &self.loopback } else { &self.remote };
        let client = connection.as_ref().map_err(Clone::clone)?;
        let headers = auth_headers(cfg, &destination)?;
        let url = format!("{}{path}", destination.base_url());
        send_json(client, rt, &url, &headers, body, stop)
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AiError {
    #[error("enter an HTTPS model address or a loopback HTTP address without embedded credentials, query or fragment")]
    Destination,
    #[error("connect a credential for this model service first")]
    Credential,
    #[error("network error: {0}")]
    Http(String),
    #[error("{message} (HTTP {code})")]
    Status { code: u16, message: String },
    #[error("unexpected answer from the model service: {0}")]
    Parse(String),
    #[error("no model available")]
    NoModel,
    /// The commander stopped the question.
    #[error("stopped")]
    Stopped,
}

impl AiError {
    /// Bad or missing API key.
    pub fn is_auth(&self) -> bool {
        matches!(self, AiError::Status { code: 400 | 401 | 403, message } if message.to_lowercase().contains("key") || message.to_lowercase().contains("auth") || message.to_lowercase().contains("permission"))
            || matches!(self, AiError::Status { code: 401 | 403, .. })
    }
}

/// One JSON request, retried after rate limits and server errors (at most twice). Setting `stop` ends it at once,
/// waits included, with [`AiError::Stopped`].
pub(crate) fn send_json(http: &reqwest::Client, rt: &Handle, url: &str, headers: &[(&str, String)], body: Option<&Value>, stop: Option<&AtomicBool>) -> Result<Value, AiError> {
    for attempt in 0..3u32 {
        if is_set(stop) {
            return Err(AiError::Stopped);
        }
        let mut req = match body {
            Some(b) => http.post(url).json(b),
            None => http.get(url),
        };
        for (k, v) in headers {
            let mut value = reqwest::header::HeaderValue::from_str(v).map_err(|_| AiError::Credential)?;
            value.set_sensitive(matches!(*k, "authorization" | "x-api-key"));
            req = req.header(*k, value);
        }
        let request = async move {
            let r = req.timeout(Duration::from_secs(120)).send().await?;
            let status = r.status().as_u16();
            let retry_after = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|s| s.trim().parse::<u64>().ok());
            let text = if r.status().is_success() { r.text().await? } else { String::new() };
            Ok::<_, reqwest::Error>((status, retry_after, text))
        };
        // Losing the race to `stop` drops the request, which cancels it.
        let res = rt.block_on(async move {
            tokio::select! {
                r = request => Some(r),
                () = until_set(stop) => None,
            }
        });
        let Some(res) = res else { return Err(AiError::Stopped) };
        match res {
            Ok((status, _, text)) if (200..300).contains(&status) => {
                return serde_json::from_str(&text).map_err(|e| AiError::Parse(e.to_string()));
            }
            Ok((status, retry_after, _)) => {
                let retryable = status >= 500 || (status == 429 && retry_after.is_some());
                if retryable && attempt < 2 {
                    pause(Duration::from_secs(retry_after.unwrap_or(2 << attempt).min(20)), stop);
                    continue;
                }
                let message = match status {
                    300..=399 => "The model service redirected the request. Check its configured address.",
                    401 | 403 => "The model service rejected authorization.",
                    404 => "The model or service path was not found.",
                    _ => "The model service rejected the request.",
                };
                return Err(AiError::Status { code: status, message: message.into() });
            }
            Err(e) if attempt < 2 && (e.is_timeout() || e.is_connect()) => {
                pause(Duration::from_secs(1 + u64::from(attempt)), stop);
            }
            Err(_) => return Err(AiError::Http("request could not be sent or read".into())),
        }
    }
    Err(AiError::Http("gave up after retries".into()))
}

fn is_set(stop: Option<&AtomicBool>) -> bool {
    stop.is_some_and(|s| s.load(Ordering::Relaxed))
}

/// Resolves once `stop` is set; never without a flag.
async fn until_set(stop: Option<&AtomicBool>) {
    match stop {
        Some(s) => {
            while !s.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        None => std::future::pending().await,
    }
}

/// Waits `d` before a retry, or less once `stop` is set.
fn pause(d: Duration, stop: Option<&AtomicBool>) {
    let end = Instant::now() + d;
    while !is_set(stop) {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        std::thread::sleep(left.min(Duration::from_millis(100)));
    }
}

/// The provider's model IDs. Discovery and completion use the same destination and stop flag.
pub fn list_models(http: &ProviderHttp, rt: &Handle, cfg: &ProviderConfig, stop: &AtomicBool) -> Result<Vec<String>, AiError> {
    let v = http.request(rt, cfg, "models", None, Some(stop))?;
    let ids = v
        .get("data")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|m| m.get("id").and_then(Value::as_str)).map(|s| s.trim_start_matches("models/").to_string()).collect())
        .unwrap_or_default();
    Ok(ids)
}

/// The configured model, else the first preferred model the provider offers, else its first model.
pub fn resolve_model(cfg: &ProviderConfig, available: &[String]) -> Option<String> {
    if !cfg.model.trim().is_empty() {
        return Some(cfg.model.trim().to_string());
    }
    let preferred = cfg.kind.preferred_models();
    preferred
        .iter()
        .find(|p| available.iter().any(|a| a == *p))
        .map(|s| s.to_string())
        .or_else(|| if available.is_empty() { preferred.first().map(|s| s.to_string()) } else { None })
        .or_else(|| available.first().cloned())
}

fn auth_headers(cfg: &ProviderConfig, destination: &Destination) -> Result<Vec<(&'static str, String)>, AiError> {
    let key = cfg.key_for(destination);
    if cfg.kind.needs_key() && key.is_none() {
        return Err(AiError::Credential);
    }
    Ok(match (cfg.kind, key) {
        (ProviderKind::Claude, Some(key)) => vec![("x-api-key", key.to_string()), ("anthropic-version", "2023-06-01".to_string())],
        (_, Some(key)) => vec![("authorization", format!("Bearer {key}"))],
        (_, None) => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_resolution() {
        let gemini = ProviderConfig { kind: ProviderKind::Gemini, ..Default::default() };
        let listed = vec!["gemini-2.5-flash".to_string(), "gemini-2.5-flash-lite".to_string()];
        assert_eq!(resolve_model(&gemini, &listed).as_deref(), Some("gemini-2.5-flash-lite"), "cheapest preferred one that exists");
        assert_eq!(resolve_model(&gemini, &[]).as_deref(), Some("gemini-3.5-flash-lite"), "no list: first preference");
        let local = ProviderConfig { kind: ProviderKind::Local, ..Default::default() };
        assert_eq!(resolve_model(&local, &["qwen3:8b".to_string()]).as_deref(), Some("qwen3:8b"));
        let chosen = ProviderConfig { model: "my-model".into(), ..gemini };
        assert_eq!(resolve_model(&chosen, &listed).as_deref(), Some("my-model"));
    }

    #[test]
    fn urls_keys_and_errors() {
        let c = ProviderConfig { kind: ProviderKind::Custom, base_url: "https://example.org/v1".into(), ..Default::default() };
        assert_eq!(c.destination().unwrap().base_url(), "https://example.org/v1/");
        assert!(!c.is_ready(), "custom services need a key");
        assert!(ProviderConfig { kind: ProviderKind::Local, ..Default::default() }.is_ready());
        let k = ProviderConfig::default();
        let destination = k.destination().unwrap();
        let k = k.with_credential(destination.clone(), "synthetic".into());
        assert!(!format!("{k:?}").contains("synthetic"));
        assert_eq!(auth_headers(&k, &destination).unwrap()[0].1, "Bearer synthetic");
        assert!(AiError::Status { code: 400, message: "API key not valid. Please pass a valid API key.".into() }.is_auth());
    }

    #[test]
    fn gemini_and_claude_ignore_a_left_over_address() {
        // Switching from a custom service back to Gemini used to keep sending Gemini's key to the custom address.
        for kind in [ProviderKind::Gemini, ProviderKind::Claude] {
            let c = ProviderConfig { kind, base_url: "https://agentrouter.org/v1".into(), ..Default::default() };
            assert_eq!(c.destination().unwrap().base_url(), kind.default_base_url(), "{kind:?}");
        }
        let local = ProviderConfig { kind: ProviderKind::Local, base_url: "http://localhost:1234/v1".into(), ..Default::default() };
        assert_eq!(local.destination().unwrap().base_url(), "http://localhost:1234/v1/", "LM Studio's port is kept");
    }

    #[test]
    fn endpoint_must_be_transport_safe() {
        for base_url in ["http://example.invalid/v1", "https://user:pass@example.invalid/v1", "https://example.invalid/v1?token=synthetic", "https://example.invalid/v1#api"] {
            let cfg = ProviderConfig { kind: ProviderKind::Custom, base_url: base_url.into(), ..Default::default() };
            assert_eq!(cfg.destination(), Err(AiError::Destination));
        }
        let cfg = ProviderConfig { kind: ProviderKind::Local, base_url: "http://example.invalid/v1".into(), ..Default::default() };
        assert!(!cfg.is_ready(), "remote model addresses require HTTPS even in Local");
    }

    #[test]
    fn one_key_is_valid_for_one_destination() {
        let custom = ProviderConfig::new(ProviderKind::Custom, "https://example.invalid/api".into(), String::new());
        let destination = custom.destination().unwrap();
        let mut config = custom.with_credential(destination.clone(), "fixture-token".into());
        assert_eq!(auth_headers(&config, &destination).unwrap()[0].1, "Bearer fixture-token");
        config.base_url = "https://example.invalid/other".into();
        assert_eq!(auth_headers(&config, &config.destination().unwrap()), Err(AiError::Credential));
        config.kind = ProviderKind::Local;
        assert!(auth_headers(&config, &config.destination().unwrap()).unwrap().is_empty());
        config.kind = ProviderKind::Claude;
        let dest = config.destination().unwrap();
        assert_eq!(auth_headers(&config, &dest), Err(AiError::Credential));
        let config = config.with_credential(dest.clone(), "claude-fixture".into());
        assert_eq!(auth_headers(&config, &dest).unwrap()[0], ("x-api-key", "claude-fixture".into()));
    }

    #[test]
    fn discovery_is_bound_and_redirects_stay_unfollowed() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;

        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        let origin = TcpListener::bind("127.0.0.1:0").unwrap();
        let other = TcpListener::bind("127.0.0.1:0").unwrap();
        other.set_nonblocking(true).unwrap();
        let address = origin.local_addr().unwrap();
        let target = other.local_addr().unwrap();
        origin.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut conn = loop {
                match origin.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
                    Err(error) => panic!("synthetic model server did not receive a request: {error}"),
                }
            };
            conn.set_nonblocking(false).unwrap();
            conn.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut reader = BufReader::new(&mut conn);
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" { break; }
                head.push_str(&line);
            }
            let body = "private-fixture-response";
            write!(conn, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{target}/models\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            head
        });
        let cfg = ProviderConfig::new(ProviderKind::Custom, format!("http://{address}/v1"), String::new());
        let cfg = cfg.clone().with_credential(cfg.destination().unwrap(), "synthetic-auth".into());
        let stop = AtomicBool::new(false);
        let err = list_models(&ProviderHttp::default(), runtime.handle(), &cfg, &stop).unwrap_err();
        assert!(matches!(err, AiError::Status { code: 307, .. }));
        assert!(!err.to_string().contains("private-fixture-response"));
        let request = server.join().unwrap();
        assert!(request.starts_with("GET /v1/models "));
        assert!(request.contains("authorization: Bearer synthetic-auth"));
        assert_eq!(other.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn stop_ends_a_request_that_hangs() {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        // A server that takes the connection and never answers.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let _held = listener.accept();
            std::thread::sleep(Duration::from_secs(30));
        });
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let http = reqwest::Client::new();
        assert_eq!(send_json(&http, rt.handle(), &url, &[], None, Some(&stop)), Err(AiError::Stopped));
        assert!(started.elapsed() < Duration::from_secs(5), "stopped after {:?}", started.elapsed());
        assert_eq!(send_json(&http, rt.handle(), &url, &[], None, Some(&stop)), Err(AiError::Stopped), "a set flag sends nothing");
    }
}
