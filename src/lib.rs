//! lightstatic —— 轻量静态文件服务器。
//!
//! 拆出 lib target 主要是为了可测试：`test/` 下的集成测试直接调用这些模块，
//! 二进制入口见 `src/main.rs`。

pub mod args;
pub mod config;
pub mod helper;
pub mod logger;
pub mod server;
pub mod server_core;
pub mod util;
