//! 模型名的本地标记处理。
//!
//! Claude Code 通过 `[1m]` 后缀声明 100 万上下文能力；上游 API 通常不接受这个本地能力
//! 标记，转发前要剥离。

use crate::claude_desktop_config::ONE_M_CONTEXT_MARKER;

/// 去掉模型名末尾的 `[1m]` 标记（大小写不敏感，标记前的空白也一并去掉）。
pub fn strip_one_m_suffix_for_upstream(model: &str) -> &str {
    let trimmed = model.trim_end();
    let marker = ONE_M_CONTEXT_MARKER.as_bytes();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= marker.len()
        && bytes[bytes.len() - marker.len()..].eq_ignore_ascii_case(marker)
    {
        return trimmed[..trimmed.len() - marker.len()].trim_end();
    }
    model
}
