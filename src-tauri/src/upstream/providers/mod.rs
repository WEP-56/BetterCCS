//! 直连用得上的上游能力：识别 Codex 官方账号、算模型目录工具档，以及各家的 OAuth 账号管理。

pub mod codex;
pub mod codex_oauth_auth;
pub mod copilot_auth;
pub(crate) mod copilot_model_map;
pub mod xai_oauth_auth;

pub const XAI_API_BASE_URL: &str = "https://api.x.ai/v1";

pub use codex::{is_codex_official_provider, resolve_codex_catalog_tool_profile};
