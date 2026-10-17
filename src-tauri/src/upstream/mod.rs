//! 上游协议的实现细节：各供应商的协议格式、请求改写、用量解析，以及出站 HTTP 客户端。
//!
//! 本地路由服务已经下线，这里剩下的都是直连时用得上的东西（把供应商行写成客户端的配置、
//! OAuth 账号管理、用量统计）。

pub mod body_filter;
pub mod cache_injector;
pub mod circuit_breaker;
pub(crate) mod content_encoding;
pub mod copilot_optimizer;
pub mod error;
pub mod error_mapper;
pub mod gemini_url;
pub mod http_client;
pub mod hyper_client;
pub(crate) mod json_canonical;
pub mod log_codes;
pub mod media_sanitizer;
pub mod model_mapper;
pub mod opaque_state_rectifier;
pub mod providers;
pub mod session;
pub(crate) mod sse;
pub mod thinking_budget_rectifier;
pub mod thinking_optimizer;
pub mod thinking_rectifier;
pub(crate) mod tool_media;
pub(crate) mod types;
pub mod usage;

// 公开导出给外部使用（commands, services等模块需要）
#[allow(unused_imports)]
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};
#[allow(unused_imports)]
pub use error::ProxyError;
#[allow(unused_imports)]
pub use session::{extract_session_id, SessionIdResult, SessionIdSource};
#[allow(unused_imports)]
pub use types::{ProxyConfig, ProxyServerInfo, ProxyStatus};

// 内部模块间共享（供子模块使用）
// 注意：这个导出用于模块内部，编译器可能警告未使用但实际被子模块使用
#[allow(unused_imports)]
pub(crate) use types::*;
