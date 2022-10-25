use async_std::path::PathBuf;
use async_std::sync::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::error;
use crate::logger::log_startup_info;
use crate::pid::{remove_pid, write_pid};
use crate::server_core::handle_request;
use crate::{args, config::Config, info};
use tide::prelude::*;

use crate::store::{init_cache_store, refresh_cache_store};
use async_std::stream::StreamExt;
use colored::Colorize;
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;

async fn handle_signals(mut signals: Signals, config: AppConfig) {
  while let Some(signal) = signals.next().await {
    info!("process got signal: {}", signal);
    match signal {
      SIGHUP => {
        // Reload configuration
        // Reopen the log file
        if let Some(store) = &config.cache_store {
          info!("Start refreshing file store");
          if let Some(size) = refresh_cache_store(store).await {
            info!("File store refreshed with {} files", size);
          } else {
            error!("Failed to refresh file store");
          }
        }
      }
      SIGTERM | SIGINT | SIGQUIT => {
        info!("{}", "lightstatic serving stopped".red());
        remove_pid();
        std::process::exit(0);
      }
      _ => unreachable!(),
    }
  }
}

pub async fn bootstrap(args: args::Args, cwd: PathBuf) -> tide::Result<()> {
  let free_port = port_selector::select_from_given_port(args.port).unwrap();
  let mut app_config = Config::new(&args, &cwd);
  let file_size = if args.cache_in_memory {
    let cache_store = init_cache_store(
      app_config.index_href.clone(),
      app_config.root_dir.clone(),
      args.regex_immutable.clone(),
    )
    .await;
    let file_size = cache_store.len();
    // info!("Init file cache store with {} files", );
    app_config.cache_store.replace(RwLock::new(cache_store));
    Some(file_size)
  } else {
    None
  };
  let app_config = Arc::new(app_config);
  let mut app = tide::with_state(app_config.clone());
  app.with(handle_request);

  let mut listener = app.bind((&args.host, free_port)).await?;
  for _ in listener.info().iter() {
    log_startup_info(&args, file_size);
  }

  let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
  let handle = signals.handle();

  let signals_task = async_std::task::spawn(handle_signals(signals, app_config));

  write_pid()?;
  // Execute your main program logic
  listener.accept().await?;

  // Terminate the signal stream.
  handle.close();
  signals_task.await;

  Ok(())
}
