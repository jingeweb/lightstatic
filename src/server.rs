use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use colored::Colorize;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::logger::log_startup_info;
use crate::server_core::handle_request;
use crate::{args, info};

pub async fn bootstrap(args: args::Args, cwd: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let free_port = port_selector::select_from_given_port(args.port).ok_or_else(|| {
    std::io::Error::other(format!(
      "no free port available starting from {}",
      args.port
    ))
  })?;

  let config = Arc::new(Config::new(&args, &cwd));
  let listener = TcpListener::bind((args.host.as_str(), free_port)).await?;
  log_startup_info(&args, free_port);

  let shutdown = shutdown_signal();
  tokio::pin!(shutdown);

  loop {
    tokio::select! {
      _ = &mut shutdown => break,
      accepted = listener.accept() => {
        let (stream, _peer) = accepted?;
        let io = TokioIo::new(stream);
        let config = config.clone();
        tokio::spawn(async move {
          let service = service_fn(move |req| {
            let config = config.clone();
            async move { Ok::<_, Infallible>(handle_request(req, config).await) }
          });
          if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
            crate::error!("connection error: {}", err);
          }
        });
      }
    }
  }

  info!("{}", "lightstatic serving stopped".red());
  Ok(())
}

/// SIGINT / SIGTERM / SIGQUIT（unix）或 Ctrl+C（windows）都表示退出。
#[cfg(unix)]
async fn shutdown_signal() {
  use tokio::signal::unix::{signal, SignalKind};

  let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
  let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
  let mut sigquit = signal(SignalKind::quit()).expect("failed to install SIGQUIT handler");

  tokio::select! {
    _ = sigterm.recv() => {}
    _ = sigint.recv() => {}
    _ = sigquit.recv() => {}
  }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
  let _ = tokio::signal::ctrl_c().await;
}
