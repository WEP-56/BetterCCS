//! 客户端配置文件的写入、设备本地状态（`live-state.json`）、写前意图和崩溃恢复。

pub mod apply;
pub mod current;
pub mod legacy;
pub mod operation;
pub mod state;
pub mod switch_lock;
