use crate::database::Database;
use crate::mode::switch_lock::SwitchLockManager;
use crate::services::UsageCache;
use crate::upstream::providers::codex_oauth_auth::CodexOAuthManager;
use std::sync::Arc;

/// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    /// 每个应用一把写入锁：同一应用的客户端文件同时只有一个操作在写。
    pub switch_locks: SwitchLockManager,
    pub usage_cache: Arc<UsageCache>,
    // 内部已使用细粒度锁（accounts/access_tokens/refresh_locks），所有方法均为
    // `&self`，无需外层 RwLock；避免持有粗粒度锁跨网络刷新导致的连锁阻塞。
    pub codex_oauth_manager: Arc<CodexOAuthManager>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let codex_oauth_manager =
            Arc::new(CodexOAuthManager::new(crate::config::get_app_config_dir()));

        Self {
            db,
            switch_locks: SwitchLockManager::new(),
            usage_cache: Arc::new(UsageCache::new()),
            codex_oauth_manager,
        }
    }
}
