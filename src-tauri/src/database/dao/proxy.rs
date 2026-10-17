//! 代理功能数据访问层
//!
//! 处理代理配置、Provider健康状态和使用统计的数据库操作

use crate::error::AppError;
use serde::{Deserialize, Serialize};

use super::super::{lock_conn, Database};

pub(crate) const PRICING_SOURCE_RESPONSE: &str = "response";
pub(crate) const PRICING_SOURCE_REQUEST: &str = "request";

fn validate_pricing_source(value: &str) -> Result<&str, AppError> {
    let trimmed = value.trim();
    if trimmed == PRICING_SOURCE_RESPONSE || trimmed == PRICING_SOURCE_REQUEST {
        Ok(trimmed)
    } else {
        Err(AppError::localized(
            "error.invalidPricingMode",
            format!("无效计费模式: {value}"),
            format!("Invalid pricing mode: {value}"),
        ))
    }
}

/// 本地路由时代接管客户端配置前留下的备份。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBackup {
    /// 应用类型（claude / codex / gemini）
    pub app_type: String,
    /// 原始配置 JSON
    pub original_config: String,
    /// 备份时间
    pub backed_up_at: String,
}

impl Database {
    // ==================== Global Proxy Config ====================

    /// 获取计费模式来源
    pub async fn get_pricing_model_source(&self, app_type: &str) -> Result<String, AppError> {
        let result = {
            let conn = lock_conn!(self.conn);
            conn.query_row(
                "SELECT pricing_model_source FROM proxy_config WHERE app_type = ?1",
                [app_type],
                |row| row.get(0),
            )
        };

        match result {
            Ok(value) => Ok(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.init_proxy_config_rows().await?;
                Ok(PRICING_SOURCE_RESPONSE.to_string())
            }
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 设置计费模式来源
    pub async fn set_pricing_model_source(
        &self,
        app_type: &str,
        value: &str,
    ) -> Result<(), AppError> {
        let trimmed = validate_pricing_source(value)?;

        // 确保行存在
        self.ensure_proxy_config_row_exists(app_type)?;

        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE proxy_config SET
                pricing_model_source = ?2,
                updated_at = datetime('now')
             WHERE app_type = ?1",
            rusqlite::params![app_type, trimmed],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 确保指定 app_type 的 proxy_config 行存在（同步版本，用于 set_* 函数）
    ///
    /// 使用与 schema.rs seed 相同的 per-app 默认值
    fn ensure_proxy_config_row_exists(&self, app_type: &str) -> Result<(), AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Lock(e.to_string()))?;

        // 根据 app_type 使用不同的默认值（与 schema.rs seed 保持一致）
        let (retries, fb_timeout, idle_timeout, cb_fail, cb_succ, cb_timeout, cb_rate, cb_min) =
            match app_type {
                "claude" => (6, 90, 180, 8, 3, 90, 0.7, 15),
                "codex" => (3, 60, 120, 4, 2, 60, 0.6, 10),
                "gemini" => (5, 60, 120, 4, 2, 60, 0.6, 10),
                "grokbuild" => (3, 60, 120, 4, 2, 60, 0.6, 10),
                _ => (3, 60, 120, 4, 2, 60, 0.6, 10), // 默认值
            };

        conn.execute(
            "INSERT OR IGNORE INTO proxy_config (
                app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests
            ) VALUES (?1, ?2, ?3, ?4, 600, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                app_type,
                retries,
                fb_timeout,
                idle_timeout,
                cb_fail,
                cb_succ,
                cb_timeout,
                cb_rate,
                cb_min
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 初始化 proxy_config 表的三行数据
    ///
    /// 使用与 schema.rs seed 相同的 per-app 默认值
    async fn init_proxy_config_rows(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        // 使用与 schema.rs seed 相同的 per-app 默认值
        // claude: 更激进的重试和超时配置
        conn.execute(
            "INSERT OR IGNORE INTO proxy_config (
                app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests
            ) VALUES ('claude', 6, 90, 180, 600, 8, 3, 90, 0.7, 15)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // codex: 默认配置
        conn.execute(
            "INSERT OR IGNORE INTO proxy_config (
                app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests
            ) VALUES ('codex', 3, 60, 120, 600, 4, 2, 60, 0.6, 10)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // gemini: 稍高的重试次数
        conn.execute(
            "INSERT OR IGNORE INTO proxy_config (
                app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests
            ) VALUES ('gemini', 5, 60, 120, 600, 4, 2, 60, 0.6, 10)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // grokbuild: Responses protocol, same timeout defaults as Codex.
        conn.execute(
            "INSERT OR IGNORE INTO proxy_config (
                app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests
            ) VALUES ('grokbuild', 3, 60, 120, 600, 4, 2, 60, 0.6, 10)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ==================== Legacy Proxy Config (兼容旧代码) ====================

    /// 设定本地路由当年用的监听端口：只在识别旧版本写下的地址、以及测试里用。
    pub fn set_proxy_listen_port_sync(&self, port: u16) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "UPDATE proxy_config SET listen_port = ?1",
                rusqlite::params![port as i32],
            );
        }
    }

    // ==================== Live Backup ====================

    /// 保存 Live 配置备份
    pub async fn save_live_backup(
        &self,
        app_type: &str,
        config_json: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO proxy_live_backup (app_type, original_config, backed_up_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![app_type, config_json, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        log::info!("已备份 {app_type} Live 配置");
        Ok(())
    }

    /// 获取 Live 配置备份
    pub async fn get_live_backup(&self, app_type: &str) -> Result<Option<LiveBackup>, AppError> {
        let conn = lock_conn!(self.conn);

        let result = conn.query_row(
            "SELECT app_type, original_config, backed_up_at FROM proxy_live_backup WHERE app_type = ?1",
            rusqlite::params![app_type],
            |row| {
                Ok(LiveBackup {
                    app_type: row.get(0)?,
                    original_config: row.get(1)?,
                    backed_up_at: row.get(2)?,
                })
            },
        );

        match result {
            Ok(backup) => Ok(Some(backup)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 删除 Live 配置备份
    pub async fn delete_live_backup(&self, app_type: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "DELETE FROM proxy_live_backup WHERE app_type = ?1",
            rusqlite::params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        log::info!("已删除 {app_type} Live 配置备份");
        Ok(())
    }

    // ==================== Sync Methods for Tray Menu ====================

    /// 同步获取应用的 proxy 启用状态和自动故障转移状态
    ///
    /// 用于托盘菜单构建等同步场景
    /// 返回 (enabled, auto_failover_enabled)
    /// 同步读客户端连代理用的主机和端口（读不到时用默认值）：本机设了客户端地址
    /// （settings.json）就用它，否则是监听地址（交给 `proxy_origin` 处理 `0.0.0.0`）。
    /// 写进客户端文件的代理地址都按这个生成，不需要代理在运行。
    pub fn get_proxy_client_endpoint_sync(&self) -> (String, u16) {
        // 本地路由时代的默认地址；表里读不到时用它们。
        let fallback = || ("127.0.0.1".to_string(), 15721u16);
        let (listen_address, port) = match self.conn.lock() {
            Ok(conn) => conn
                .query_row(
                    "SELECT listen_address, listen_port FROM proxy_config WHERE app_type = 'claude'",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? as u16)),
                )
                .unwrap_or_else(|_| fallback()),
            Err(_) => fallback(),
        };
        let host = crate::settings::get_proxy_client_host().unwrap_or(listen_address);
        (host, port)
    }

    pub fn get_proxy_flags_sync(&self, app_type: &str) -> (bool, bool) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return (false, false),
        };

        conn.query_row(
            "SELECT enabled, auto_failover_enabled FROM proxy_config WHERE app_type = ?1",
            [app_type],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get::<_, i32>(1)? != 0)),
        )
        .unwrap_or((false, false))
    }

    /// 同步设置应用的 proxy 启用状态和自动故障转移状态
    ///
    /// 用于托盘菜单点击等同步场景
    pub fn set_proxy_flags_sync(
        &self,
        app_type: &str,
        enabled: bool,
        auto_failover_enabled: bool,
    ) -> Result<(), AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Mutex lock failed: {e}")))?;

        conn.execute(
            "UPDATE proxy_config SET enabled = ?2, auto_failover_enabled = ?3, updated_at = datetime('now') WHERE app_type = ?1",
            rusqlite::params![
                app_type,
                if enabled { 1 } else { 0 },
                if auto_failover_enabled { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::error::AppError;

    #[tokio::test]
    async fn test_pricing_model_source_round_trip_and_validation() -> Result<(), AppError> {
        let db = Database::memory()?;

        let default = db.get_pricing_model_source("claude").await?;
        assert_eq!(default, "response");

        db.set_pricing_model_source("claude", "request").await?;
        let updated = db.get_pricing_model_source("claude").await?;
        assert_eq!(updated, "request");

        let err = db
            .set_pricing_model_source("claude", "invalid")
            .await
            .unwrap_err();
        // AppError::localized returns AppError::Localized variant
        assert!(matches!(
            err,
            AppError::Localized {
                key: "error.invalidPricingMode",
                ..
            }
        ));

        Ok(())
    }
}
