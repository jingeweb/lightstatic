use crate::{
  args,
  util::{self, resolve_path},
};

use colored::Colorize;
use fern::{DateBased, Dispatch};
use log::LevelFilter;
use std::fmt::Display;
use std::path::Path;

pub fn initialize_log(args: &args::Args, cwd: &Path) {
  let mut info_dest = Dispatch::new()
    .level(LevelFilter::Off)
    .level_for("lightstatic::info", LevelFilter::Error);
  // .format(|out, msg, rec| {
  //   out.finish(format_args!("{} {}", rec.target(), msg))
  // });
  let mut error_dest = Dispatch::new()
    .level(LevelFilter::Off)
    .level_for("lightstatic::error", LevelFilter::Error);
  let mut access_dest = Dispatch::new()
    .level(LevelFilter::Off)
    .level_for("lightstatic::access", LevelFilter::Error);
  let mut logger = Dispatch::new().level(LevelFilter::Error);
  //.level_for("lightstatic::info", LevelFilter::Error).level_for("lightstatic::error", LevelFilter::Error)
  if let Some(log_dir) = &args.log_dir {
    let prefix = resolve_path(cwd, Path::new(log_dir));

    if !std::path::Path::new(&prefix).is_dir() {
      panic!("--log-dir not exists or is not directory");
    }
    info_dest = info_dest
      .chain(DateBased::new(format!("{}/info.", prefix.display()), "%Y-%m-%d.log").utc_time());
    error_dest = error_dest
      .chain(DateBased::new(format!("{}/error.", prefix.display()), "%Y-%m-%d.log").utc_time());
    access_dest = access_dest
      .chain(DateBased::new(format!("{}/access.", prefix.display()), "%Y-%m-%d.log").utc_time());

    logger = logger.format(|out, message, _| {
      out.finish(format_args!(
        "{} {}",
        chrono::Utc::now().format("[%H:%M:%S]"),
        message
      ))
    })
  } else {
    access_dest = access_dest.chain(std::io::stdout());
    info_dest = info_dest.chain(std::io::stdout());
    error_dest = error_dest.chain(std::io::stderr());
  }
  logger
    .chain(access_dest)
    .chain(info_dest)
    .chain(error_dest)
    .apply()
    .unwrap();
}

#[macro_export]
macro_rules! info {
  ($($arg:tt)+) => (log::log!(target: "lightstatic::info", log::Level::Error, $($arg)+))
}

#[macro_export]
macro_rules! error {
  ($($arg:tt)+) => (log::log!(target: "lightstatic::error", log::Level::Error, $($arg)+))
}

fn log_listening<T: Display>(ip: T, port: u16, base: &Option<String>) {
  if let Some(x) = base.as_ref() {
    if !util::is_empty_root_url(x) {
      return info!("  http://{}:{}  with base href {}", ip, port, x.yellow());
    }
  };
  info!("  http://{}:{}", ip, port);
}

pub fn log_startup_info(args: &args::Args, port: u16) {
  info!(
    "{}{}",
    "Starting up lightstatic, serving: ".yellow(),
    args.serve_path.as_ref().unwrap().cyan(),
  );
  info!("{}", "Available on:".yellow());
  let bind_all = args.host.eq("0.0.0.0");
  if bind_all {
    match local_ip_address::local_ip() {
      Ok(ip) => log_listening(ip, port, &args.base_href),
      Err(err) => error!("{} {}", "Could not detect a LAN address:".yellow(), err),
    }
    log_listening("127.0.0.1", port, &args.base_href);
  } else {
    log_listening(&args.host, port, &args.base_href);
  }
  if args.html5 {
    info!(
      "Html5 route mode rewrite to {}",
      crate::config::INDEX_FILE.cyan()
    );
  }
  info!("Press {} to stop it.", "ctrl+c".yellow());
  if args.open {
    let host = if bind_all {
      "127.0.0.1"
    } else {
      args.host.as_str()
    };
    let url = format!("http://{}:{}", host, port);
    if let Err(err) = open::that(&url) {
      error!("Failed to open browser at {}: {}", url, err);
    }
  }
}

pub fn log_access(status: u16, url: &str) {
  log::log!(target: "lightstatic::access", log::Level::Error, "{} {} {}", "GET".yellow(), if status < 400 {
    status.to_string().green()
  } else {
    status.to_string().red()
  }, url.cyan());
}
