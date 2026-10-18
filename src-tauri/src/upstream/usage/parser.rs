//! 会话日志里的 token 用量。
//!
//! 直连不改写入站响应，用量一律从各家自己写的会话日志里汇总（见
//! `services::session_usage`），这里只放共用的结构和字段解析。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Claude usage 中按 1 小时 TTL 写入的缓存 token（`cache_creation_input_tokens` 的子集）
pub(crate) fn claude_cache_creation_1h_tokens(usage: &Value) -> u32 {
    usage
        .pointer("/cache_creation/ephemeral_1h_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32
}

/// Session 日志 request_id 前缀，与 `session_usage.rs` 中的格式保持一致
pub const SESSION_REQUEST_ID_PREFIX: &str = "session:";

/// Token 使用量统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
    /// `cache_creation_tokens` 中按 1 小时 TTL 写入的部分（Claude 单独计价）
    pub cache_creation_1h_tokens: u32,
    /// 从响应中提取的实际模型名称（如果可用）
    pub model: Option<String>,
}
