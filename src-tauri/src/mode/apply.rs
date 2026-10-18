//! 把当前供应商写进客户端的配置文件。
//!
//! 客户端文件只有一种形态：直连指针那家的配置。以前还支持「本地路由」——把客户端指向一个
//! 本地端口，由本地服务做协议转换。本地路由下线后，那些还指着本地端口的客户端要写回直连，
//! 见 [`startup`]。
//!
//! 所有写入都经 `services::provider::*_direct` 的写入引擎：写前记下意图，文件和状态记录在同
//! 一个操作里提交，崩溃后按 pending 前滚（`mode::operation`）。

use std::fmt::Display;

use tokio::sync::OwnedMutexGuard;

use crate::app_config::AppType;
use crate::claude_desktop_config::{self, DesktopTarget};
use crate::error::AppError;
use crate::live::engine::DeviceStore;
use crate::live::project::claude::{direct_patch, ClaudeProjection};
use crate::live::project::gemini::GeminiProjection;
use crate::live::project::grok::GrokProjection;
use crate::provider::Provider;
use crate::services::provider::codex_direct::{self, Owner};
use crate::services::provider::{claude_direct, gemini_direct, grok_direct};
use crate::store::AppState;

use super::current;
use super::legacy;
use super::operation;
use super::state::{op, Contract, Mode, ModeState, PendingTarget};

/// 有客户端配置文件的应用。Claude Desktop 只在启动时读一次文件。
pub const APPS: [AppType; 5] = [
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::GrokBuild,
    AppType::ClaudeDesktop,
];

fn err(error: impl Display) -> String {
    error.to_string()
}

fn direct_provider(state: &AppState, app: &AppType) -> Result<Option<Provider>, String> {
    current::direct_provider(&state.db, app).map_err(err)
}

/// 客户端文件现在是谁写进去的。
///
/// `Direct` 装的是整张供应商行（一千多字节），装箱是为了别让这个短命枚举每次都在栈上摊开。
enum LiveNow {
    /// 直连投影（或还没写过任何东西）。
    Direct(Option<Box<Provider>>),
    /// 本地路由时代留下的契约；`contract` 是当时记下的独有字段，写回直连时按它删除。
    Legacy(Option<Contract>),
}

impl LiveNow {
    fn of(state: &AppState, app: &AppType, mode: &ModeState) -> Result<Self, String> {
        if mode.attached || mode.is_proxy() {
            Ok(Self::Legacy(mode.contract.clone()))
        } else {
            Ok(Self::Direct(direct_provider(state, app)?.map(Box::new)))
        }
    }

    /// 客户端里现在由哪个供应商带进来的独有字段。
    fn claude_exclusive_owner(&self) -> Option<ClaudeProjection> {
        match self {
            Self::Direct(provider) => provider
                .as_ref()
                .map(|provider| ClaudeProjection::of(&provider.settings_config)),
            Self::Legacy(Some(contract)) => Some(ClaudeProjection {
                exclusive: contract.exclusive.clone(),
                ..ClaudeProjection::default()
            }),
            Self::Legacy(None) => None,
        }
    }

    /// Codex 的客户端文件现在是谁写进去的。
    fn codex_owner(&self) -> Owner<'_> {
        match self {
            Self::Direct(provider) => provider.as_deref().map_or(Owner::None, Owner::Provider),
            Self::Legacy(Some(contract)) => Owner::Contract {
                contract,
                route: None,
            },
            Self::Legacy(None) => Owner::None,
        }
    }
}

/// 只落定状态，不碰客户端文件。
fn commit_state(state: &AppState, app: &AppType, target: &PendingTarget) -> Result<(), String> {
    operation::commit_target(&state.db, &DeviceStore::for_device(), app.as_str(), target)
        .map_err(err)
}

/// 写回直连投影（直连指针的供应商）。
fn write_direct(
    state: &AppState,
    app: &AppType,
    op_name: &str,
    live_now: &LiveNow,
    target: ModeState,
) -> Result<(), String> {
    let direct = direct_provider(state, app)?;
    let pending_target = PendingTarget::mode(target);
    let attached = matches!(live_now, LiveNow::Legacy(_));
    match app {
        AppType::Claude => {
            let empty = ClaudeProjection::default();
            let projection = usable_direct(app, direct.as_ref())
                .map(|provider| ClaudeProjection::of(&provider.settings_config));
            let patch = direct_patch(
                live_now.claude_exclusive_owner().as_ref(),
                projection.as_ref().unwrap_or(&empty),
            );
            claude_direct::run(
                &state.db,
                op_name,
                attached.then_some(&patch),
                pending_target,
            )
            .map_err(err)?;
        }
        AppType::Codex => {
            if !attached {
                commit_state(state, app, &pending_target)?;
                return Ok(());
            }
            let target = usable_direct(app, direct.as_ref());
            let owner = live_now.codex_owner();
            if let Err(error) = codex_direct::write_direct(
                &state.db,
                &state.codex_oauth_manager,
                op_name,
                owner,
                target,
                pending_target.clone(),
            ) {
                // 直连供应商写不出来（比如绑定的托管账号已被删除）也不能让客户端一直指着
                // 本地路由：退一步只清空关键字段。
                log::warn!("写回直连的 Codex 配置失败，只清空关键字段: {error}");
                codex_direct::write_direct(
                    &state.db,
                    &state.codex_oauth_manager,
                    op_name,
                    owner,
                    None,
                    pending_target,
                )
                .map_err(err)?;
            }
        }
        AppType::Gemini => {
            let projection = attached.then(|| {
                direct_or_empty(
                    app,
                    direct.as_ref(),
                    gemini_direct::projection,
                    GeminiProjection::empty,
                )
            });
            gemini_direct::run(&state.db, op_name, projection.as_ref(), pending_target)
                .map_err(err)?;
        }
        AppType::GrokBuild => {
            let projection = attached.then(|| {
                direct_or_empty(app, direct.as_ref(), grok_direct::projection, || {
                    GrokProjection { table: None }
                })
            });
            grok_direct::run(
                &state.db,
                op_name,
                None,
                projection.as_ref(),
                pending_target,
            )
            .map_err(err)?;
        }
        AppType::ClaudeDesktop => {
            // 直连那家写不进去（模型映射卡、配置不全）也不能让 Desktop 一直指着本地路由：
            // 退回官方。
            let profile = if attached {
                claude_desktop_config::direct_target(direct.as_ref()).unwrap_or_else(|error| {
                    log::warn!("写回直连的 Claude Desktop 配置失败，改为官方: {error}");
                    None
                })
            } else {
                None
            };
            let write = attached.then_some(match &profile {
                Some(profile) => DesktopTarget::Profile(profile),
                None => DesktopTarget::Official,
            });
            claude_desktop_config::run(&state.db, op_name, write, pending_target).map_err(err)?;
        }
        _ => return Err(format!("{} 没有客户端配置文件", app.as_str())),
    }
    Ok(())
}

/// 直连供应商的投影。写不出来（没有、行里带着占位符、缺 Key）也不能让客户端一直指着本地
/// 路由：退一步只清空关键字段（`empty`）。
fn direct_or_empty<P>(
    app: &AppType,
    direct: Option<&Provider>,
    project: impl FnOnce(&Provider) -> Result<P, AppError>,
    empty: impl FnOnce() -> P,
) -> P {
    usable_direct(app, direct)
        .map(project)
        .transpose()
        .unwrap_or_else(|error| {
            log::warn!(
                "写回直连的 {} 配置失败，只清空关键字段: {error}",
                app.as_str()
            );
            None
        })
        .unwrap_or_else(empty)
}

/// 能照写回客户端的直连供应商：行里本身带着本地路由占位符（旧版接管期间被导入的残留）的
/// 不行，否则客户端会一直指着已经不在的本地端口。
fn usable_direct<'a>(app: &AppType, direct: Option<&'a Provider>) -> Option<&'a Provider> {
    direct.filter(|provider| {
        let polluted = legacy::config_taken_over(app, &provider.settings_config);
        if polluted {
            log::warn!(
                "直连供应商 {} 的行里带着本地路由占位符，只清空关键字段",
                provider.id
            );
        }
        !polluted
    })
}

/// 拿这个应用的写入锁，再补完它上一次没做完的写入。之后读到的指针和写入记录都是落定过的。
///
/// 补不完（比如本机设置文件写不进去）就拒绝这次操作：这时读到的还是补完前的指针，照着做
/// 下去（比如只存了一行、以为它不是当前供应商），等那次操作补完就和刚做的对不上了。
pub(crate) async fn lock_settled(
    state: &AppState,
    app: &AppType,
) -> Result<OwnedMutexGuard<()>, AppError> {
    let guard = state.switch_locks.lock_for_app(app.as_str()).await;
    operation::settle(&state.db, app.as_str()).map_err(|error| {
        AppError::localized(
            "mode.unsettled",
            format!(
                "{} 上一次写配置文件的操作没做完，现在也补不完：{error}。本次什么都没做，请排查后重试",
                app.as_str()
            ),
            format!(
                "The previous write to {}'s config files is unfinished and cannot be completed now: {error}. Nothing was done; fix the cause and retry",
                app.as_str()
            ),
        )
    })?;
    Ok(guard)
}

/// 同步代码里用的 [`lock_settled`]。
pub(crate) fn lock_settled_blocking(
    state: &AppState,
    app: &AppType,
) -> Result<Option<OwnedMutexGuard<()>>, AppError> {
    if !app.has_route_mode() {
        return Ok(None);
    }
    futures::executor::block_on(lock_settled(state, app)).map(Some)
}

/// 启动时清理本地路由时代的接管态：客户端还指着本地端口、或者状态记录说接上了路由的，
/// 一律写回直连。已经直连的应用不动。
pub async fn startup(state: &AppState) {
    for app in APPS {
        let result = match lock_settled(state, &app).await {
            Ok(_guard) => reset_legacy(state, &app).await,
            Err(error) => Err(error.to_string()),
        };
        if let Err(error) = result {
            log::error!("启动时清理 {} 的本地路由接管态失败: {error}", app.as_str());
        }
    }
}

async fn reset_legacy(state: &AppState, app: &AppType) -> Result<(), String> {
    let mut mode = current::mode_state(app);
    let had_backup = drain_legacy_backup(state, app).await;
    let taken_over = legacy::live_taken_over(&state.db, app);
    let stale = taken_over || had_backup || mode.attached || mode.is_proxy();
    if !stale {
        return Ok(());
    }
    if taken_over {
        // 客户端里还是旧契约：按「已接上」处理，下面的投影会整体换掉它。
        mode.attached = true;
    }
    let live_now = LiveNow::of(state, app, &mode)?;
    write_direct(
        state,
        app,
        op::EXIT,
        &live_now,
        ModeState {
            mode: Some(Mode::Direct),
            attached: false,
            proxy_route: None,
            contract: None,
        },
    )?;
    log::info!(
        "[MODE] {} 的客户端配置还指着本地路由，已写回直连",
        app.as_str()
    );
    Ok(())
}

/// 旧版的接管备份行：不回放，转存到本机文件后删除。留着的话，降级回旧版时会把这份陈旧的
/// 快照写回客户端。
async fn drain_legacy_backup(state: &AppState, app: &AppType) -> bool {
    let backup = match state.db.get_live_backup(app.as_str()).await {
        Ok(Some(backup)) => backup,
        Ok(None) => return false,
        Err(error) => {
            log::warn!("读取 {} 的旧接管备份失败: {error}", app.as_str());
            return false;
        }
    };
    let dir = crate::config::get_home_dir()
        .join(".cc-switch")
        .join("backups")
        .join("proxy-live-backup");
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let path = dir.join(format!("{}-{stamp}.json", app.as_str()));
    let saved = serde_json::to_vec_pretty(&serde_json::json!({
        "app": app.as_str(),
        "backedUpAt": backup.backed_up_at,
        "originalConfig": serde_json::from_str::<serde_json::Value>(&backup.original_config)
            .unwrap_or(serde_json::Value::String(backup.original_config.clone())),
    }))
    .map_err(err)
    .and_then(|bytes| {
        std::fs::create_dir_all(&dir).map_err(err)?;
        crate::config::atomic_write_private(&path, &bytes).map_err(err)
    });
    match saved {
        Ok(()) => {
            if let Err(error) = state.db.delete_live_backup(app.as_str()).await {
                log::warn!("删除 {} 的旧接管备份失败: {error}", app.as_str());
            } else {
                log::info!(
                    "{} 的旧接管备份已转存到 {} 并从数据库删除",
                    app.as_str(),
                    path.display()
                );
            }
        }
        Err(error) => log::warn!(
            "转存 {} 的旧接管备份失败，保留数据库里的备份行: {error}",
            app.as_str()
        ),
    }
    true
}
