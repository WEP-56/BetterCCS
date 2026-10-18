//! Codex 供应商身份判定：是不是官方账号、该用哪一档模型目录工具。
//!
//! 本地路由下线后，这里不再有协议转换：直连时 BetterCCS 只是把供应商行写进
//! `~/.codex/config.toml` 和模型目录，请求由 Codex 自己发。

use crate::provider::Provider;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

/// 这家 Codex 供应商的配置里显式写了第三方上游（自建地址、Bearer token、非 `openai`
/// 的 `model_provider`）。官方卡不会写这些。
fn has_explicit_codex_third_party_upstream(provider: &Provider) -> bool {
    let non_empty_setting = |key: &str| {
        provider
            .settings_config
            .get(key)
            .and_then(JsonValue::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    };
    let config = provider
        .settings_config
        .get("config")
        .and_then(JsonValue::as_str)
        .map(|text| {
            crate::codex_config::strip_codex_unified_session_bucket(text)
                .unwrap_or_else(|_| text.to_string())
        });
    let config = config.as_deref();

    ["baseUrl", "baseURL", "base_url"]
        .into_iter()
        .any(non_empty_setting)
        || config
            .and_then(crate::codex_config::extract_codex_experimental_bearer_token)
            .is_some()
        || config
            .and_then(crate::codex_config::extract_codex_base_url)
            .is_some()
        || config
            .and_then(|text| text.parse::<TomlValue>().ok())
            .and_then(|doc| {
                doc.get("model_provider")
                    .and_then(TomlValue::as_str)
                    .map(str::trim)
                    .filter(|provider_id| !provider_id.is_empty())
                    .map(str::to_string)
            })
            // 和上游一致按精确匹配：内置查表区分大小写，所以 `OpenAI` 指向自建表，
            // 那是第三方上游，不是官方。
            .is_some_and(|provider_id| provider_id != "openai")
}

/// Codex 官方 ChatGPT 卡的登录由调用它的 Codex 客户端提供（`requires_openai_auth = true`）。
/// 没绑托管账号、但存了 API Key 的卡走直连 OpenAI API，不发到 ChatGPT 后端。
pub fn is_codex_official_provider(provider: &Provider) -> bool {
    let is_fixed_official_id = provider.id == crate::database::CODEX_OFFICIAL_PROVIDER_ID;
    if is_fixed_official_id && provider.category.as_deref() == Some("official") {
        return true;
    }

    let has_auth_object = provider
        .settings_config
        .get("auth")
        .is_some_and(JsonValue::is_object);
    let has_valid_config_shape = provider
        .settings_config
        .get("config")
        .is_none_or(|config| config.is_null() || config.is_string());
    if !has_auth_object || !has_valid_config_shape {
        return false;
    }

    if has_explicit_codex_third_party_upstream(provider) {
        return false;
    }

    let has_managed_account = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.managed_account_id_for("codex_oauth"))
        .is_some_and(|account_id| !account_id.trim().is_empty());
    if has_managed_account {
        return true;
    }

    let has_stored_api_key = provider
        .settings_config
        .get("auth")
        .and_then(|auth| auth.get("OPENAI_API_KEY"))
        .and_then(JsonValue::as_str)
        .is_some_and(|key| !key.trim().is_empty());
    if has_stored_api_key {
        return false;
    }

    is_fixed_official_id || provider.category.as_deref() == Some("official")
}

/// 官方 Codex 集成是原生 `/responses` 网关、且拒收 Codex 自由格式自定义工具
/// （`apply_patch` 的 `type: "custom"`，#6944）的厂商。按主机名匹配，不做子串匹配。
const CODEX_NATIVE_RESPONSES_HOSTS: &[&str] = &[
    "bigmodel.cn",
    "z.ai",
    "xiaomimimo.com",
    "minimaxi.com",
    "minimax.cn",
    "minimax.io",
    "longcat.chat",
];

/// 上述厂商「OpenAI Chat Completions」端点的路径标记，那不是它的 Responses 端点。
/// 存着 Chat 路径的行是 2026-09 之前的 Chat 路由记录，不当作原生 Responses。
const CODEX_NATIVE_RESPONSES_CHAT_PATH_MARKERS: &[&str] = &["/paas/v4"];

/// `base_url` 是不是这些厂商的原生 Responses 网关。
fn is_codex_native_responses_url(base_url: &str) -> bool {
    if !crate::codex_config::codex_url_host_matches_any(base_url, CODEX_NATIVE_RESPONSES_HOSTS) {
        return false;
    }
    if base_url
        .trim_end_matches('/')
        .to_ascii_lowercase()
        .ends_with("/chat/completions")
    {
        return false;
    }
    let lower = base_url.to_ascii_lowercase();
    !CODEX_NATIVE_RESPONSES_CHAT_PATH_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// 这家 Codex 供应商的真实上游说的是原生 Anthropic Messages 协议（`/v1/messages`）。
///
/// 只看显式配置（`api_format` / `wire_api`），不猜地址：Anthropic 网关地址五花八门，猜很容易猜错。
fn codex_provider_uses_anthropic(provider: &Provider) -> bool {
    if let Some(api_format) = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.api_format.as_deref())
        .or_else(|| {
            provider
                .settings_config
                .get("api_format")
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            provider
                .settings_config
                .get("apiFormat")
                .and_then(|v| v.as_str())
        })
    {
        return is_anthropic_wire_api(api_format);
    }

    provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .and_then(extract_codex_wire_api_from_toml)
        .map(|wire_api| is_anthropic_wire_api(&wire_api))
        .unwrap_or(false)
}

/// 算模型目录该用哪一档工具（见 `CodexCatalogToolProfile`）。
///
/// Anthropic 判定和留在客户端文件里的协议保持一致，生成的目录才不会把自由格式的
/// `apply_patch` 工具发给说 Anthropic 协议的上游。
pub fn resolve_codex_catalog_tool_profile(
    provider: &Provider,
) -> crate::codex_config::CodexCatalogToolProfile {
    use crate::codex_config::CodexCatalogToolProfile;
    if is_codex_official_provider(provider) {
        return CodexCatalogToolProfile::NativeResponses;
    }
    // xAI OAuth 固定用原生 Responses 档，不受可编辑的 api_format 影响。
    if provider.is_xai_oauth() {
        return CodexCatalogToolProfile::NativeResponses;
    }
    if provider.is_github_copilot() {
        return CodexCatalogToolProfile::Copilot;
    }
    if codex_provider_uses_anthropic(provider) {
        return CodexCatalogToolProfile::Anthropic;
    }

    // 预设切到 `openai_responses` 之前存进数据库的行：`apiFormat` 是旧的，但地址已经
    // 指向厂商的原生 Responses 网关，按 NativeResponses 算。Chat 路径不在此列。
    if let Some(base_url) = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .and_then(extract_codex_base_url_from_toml)
        .or_else(|| {
            provider
                .settings_config
                .get("base_url")
                .or_else(|| provider.settings_config.get("baseURL"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
    {
        if is_codex_native_responses_url(&base_url) {
            return CodexCatalogToolProfile::NativeResponses;
        }
    }

    let api_format = provider
        .meta
        .as_ref()
        .and_then(|m| m.api_format.as_deref())
        .or_else(|| {
            provider
                .settings_config
                .get("api_format")
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            provider
                .settings_config
                .get("apiFormat")
                .and_then(|v| v.as_str())
        });
    CodexCatalogToolProfile::from_api_format(api_format)
}

fn is_anthropic_wire_api(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "anthropic" | "anthropic_messages" | "anthropic-messages" | "claude" | "messages"
    )
}

fn extract_codex_wire_api_from_toml(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<TomlValue>().ok()?;

    if let Some(active_provider) = doc.get("model_provider").and_then(|v| v.as_str()) {
        if let Some(wire_api) = doc
            .get("model_providers")
            .and_then(|providers| providers.get(active_provider))
            .and_then(|provider| provider.get("wire_api"))
            .and_then(|v| v.as_str())
        {
            return Some(wire_api.to_string());
        }
    }

    doc.get("wire_api")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn extract_codex_base_url_from_toml(config_text: &str) -> Option<String> {
    // 规范实现在 codex_config，这里保持薄别名，两处共用同一份解析。
    crate::codex_config::extract_codex_base_url(config_text)
}
