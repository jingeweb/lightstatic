use async_std::path::PathBuf;
use std::sync::Arc;

use crate::logger::log_startup_info;
use crate::server_core::handle_request;
use crate::{args, config::Config};
use tide::prelude::*;

// 优雅退出只在 unix 上做：signal-hook-async-std 依赖 unix 的 UnixStream，
// Windows 上按控制台默认行为处理 Ctrl+C 即可。
#[cfg(unix)]
use async_std::stream::StreamExt;
#[cfg(unix)]
use colored::Colorize;
#[cfg(unix)]
use signal_hook::consts::signal::{SIGINT, SIGQUIT, SIGTERM};
#[cfg(unix)]
use signal_hook_async_std::Signals;

#[cfg(unix)]
fn shutdown() -> ! {
  crate::info!("{}", "lightstatic serving stopped".red());
  std::process::exit(0);
}

#[cfg(unix)]
async fn handle_signals(mut signals: Signals) {
  while let Some(signal) = signals.next().await {
    crate::info!("process got signal: {}", signal);
    if signal == SIGINT || signal == SIGTERM || signal == SIGQUIT {
      shutdown();
    }
  }
}

pub async fn bootstrap(args: args::Args, cwd: PathBuf) -> tide::Result<()> {
  let free_port = port_selector::select_from_given_port(args.port).ok_or_else(|| {
    http_types::Error::new(
      http_types::StatusCode::InternalServerError,
      std::io::Error::other(format!(
        "no free port available starting from {}",
        args.port
      )),
    )
  })?;

  let app_config = Arc::new(Config::new(&args, &cwd));
  let mut app = tide::with_state(app_config);
  app.with(handle_request);

  let mut listener = app.bind((&args.host, free_port)).await?;
  log_startup_info(&args, free_port);

  #[cfg(unix)]
  let signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;
  #[cfg(unix)]
  let signals_handle = signals.handle();
  #[cfg(unix)]
  let signals_task = async_std::task::spawn(handle_signals(signals));

  // Execute your main program logic
  listener.accept().await?;

  // Terminate the signal stream.
  #[cfg(unix)]
  {
    signals_handle.close();
    signals_task.await;
  }

  Ok(())
}
