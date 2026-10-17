//! 识别「本地路由时代」留下的接管态。
//!
//! 以前的版本会把客户端配置文件指向本地路由端口，并把凭据换成占位符
//! （`PROXY_TOKEN_PLACEHOLDER`）。路由下线后这种配置会让客户端连一个不存在的端口，
//! 启动时要把它们认出来并写回直连。
//!
//! 这里只读，不改文件。

use serde_json::Value;

use crate::app_config::AppType;
use crate::database::Database;
use crate::live::project::claude::PROXY_TOKEN_PLACEHOLDER;

/// 客户端配置里有没有接管占位符：有这个占位符的配置是本地路由时代写的。
///
/// 也用于供应商行：旧版接管期间被导入的行里同样带着占位符，不能照写回客户端，
/// 否则客户端会一直指着已经不在的本地端口。
pub(crate) fn config_taken_over(app: &AppType, config: &Value) -> bool {
    match app {
        AppType::Claude => claude_taken_over(config),
        AppType::Codex => codex_taken_over(config),
        AppType::Gemini => gemini_taken_over(config),
        AppType::GrokBuild => grok_taken_over(config),
        _ => false,
    }
}

/// 客户端文件里有没有接管占位符。Codex 的官方路由不带占位符，按指向本地代理的地址认。
pub(crate) fn live_taken_over(db: &Database, app: &AppType) -> bool {
    match app {
        AppType::Claude => read_json(&crate::config::get_claude_settings_path())
            .is_some_and(|config| claude_taken_over(&config)),
        AppType::Codex => crate::codex_config::read_codex_live_settings().is_ok_and(|config| {
            codex_taken_over(&config)
                || config
                    .get("config")
                    .and_then(Value::as_str)
                    .is_some_and(|text| {
                        crate::services::provider::codex_direct::routes_official_to_proxy(db, text)
                    })
        }),
        AppType::Gemini => read_gemini().is_some_and(|config| gemini_taken_over(&config)),
        AppType::GrokBuild => crate::grok_config::read_grok_live_settings()
            .is_ok_and(|config| grok_taken_over(&config)),
        _ => false,
    }
}

fn claude_taken_over(config: &Value) -> bool {
    let Some(env) = config.get("env").and_then(Value::as_object) else {
        return false;
    };
    [
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "OPENROUTER_API_KEY",
        "OPENAI_API_KEY",
    ]
    .iter()
    .any(|key| env.get(*key).and_then(Value::as_str) == Some(PROXY_TOKEN_PLACEHOLDER))
}

fn codex_taken_over(config: &Value) -> bool {
    let placeholder = config
        .get("auth")
        .and_then(Value::as_object)
        .and_then(|auth| auth.get("OPENAI_API_KEY"))
        .and_then(Value::as_str)
        == Some(PROXY_TOKEN_PLACEHOLDER);
    placeholder
        || config
            .get("config")
            .and_then(Value::as_str)
            .is_some_and(|text| {
                crate::codex_config::extract_codex_experimental_bearer_token(text).as_deref()
                    == Some(PROXY_TOKEN_PLACEHOLDER)
                    || crate::codex_config::codex_config_has_official_proxy_route(text)
            })
}

fn gemini_taken_over(config: &Value) -> bool {
    config
        .get("env")
        .and_then(Value::as_object)
        .and_then(|env| env.get("GEMINI_API_KEY"))
        .and_then(Value::as_str)
        == Some(PROXY_TOKEN_PLACEHOLDER)
}

fn grok_taken_over(config: &Value) -> bool {
    config
        .get("config")
        .and_then(Value::as_str)
        .is_some_and(|toml| crate::grok_config::has_proxy_placeholder(toml, PROXY_TOKEN_PLACEHOLDER))
}

fn read_json(path: &std::path::Path) -> Option<Value> {
    crate::config::read_json_file(path).ok().filter(Value::is_object)
}

fn read_gemini() -> Option<Value> {
    use crate::gemini_config::{env_to_json, get_gemini_env_path, read_gemini_env};
    if !get_gemini_env_path().exists() {
        return None;
    }
    let env = read_gemini_env().ok()?;
    Some(env_to_json(&env))
}

/// 本地路由的地址（`http://host:port`）。只在识别和清理旧版本写的地址时用。
pub(crate) fn origin(host: &str, listen_port: u16) -> String {
    let host = match host {
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        other => other,
    };
    if host.contains(':') && !host.starts_with('[') {
        format!("http://[{host}]:{listen_port}")
    } else {
        format!("http://{host}:{listen_port}")
    }
}
