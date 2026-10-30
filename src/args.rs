use clap::Parser;
use std::process;

use crate::util::is_empty_root_url;

#[derive(Parser, Debug)]
#[clap(version)]
pub struct Args {
  /// directory path to serve
  #[clap(value_parser, value_name = "PATH")]
  pub serve_path: Option<String>,
  /// ip address to bind (default: "0.0.0.0")
  #[clap(
    short = 'H',
    long,
    value_parser,
    value_name = "IP",
    default_value = "0.0.0.0"
  )]
  pub host: String,
  /// port to listen (default: "8080"). if the specified port is not available, find a free port instead.
  #[clap(short, long, value_parser, default_value_t = 8080)]
  pub port: u16,
  /// gzip encode response content (default: false)
  #[clap(short, long, value_parser, default_value_t = false)]
  pub gzip: bool,
  /// open browser window after starting the server (default: false)
  #[clap(short, long, value_parser, default_value_t = false)]
  pub open: bool,
  /// use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
  #[clap(short = '5', long, value_parser, default_value_t = false)]
  pub html5: bool,
  /// list directories even when they contain index.html (default: false)
  #[clap(long, value_parser, default_value_t = false)]
  pub list_dir: bool,
  /// delay in milliseconds for response (default: "0")
  #[clap(short, long, value_parser, default_value_t = 0)]
  pub delay: u64,
  /// write logs to directory, if specified.
  #[clap(short, long, value_parser, value_name = "DIRECTORY")]
  pub log_dir: Option<String>,
  /// server base href, useful when under nginx sub path
  #[clap(short, long, value_parser)]
  pub base_href: Option<String>,
  /// do not print access log
  #[clap(short = 'A', long, value_parser, default_value_t = false)]
  pub no_access: bool,
  /// disable color log
  #[clap(short = 'C', long, value_parser, default_value_t = false)]
  pub no_color: bool,
}

/// 把 `--base-href` 规范化为 `"/xxx/"` 形式；空值/`"/"` 视为未设置。
pub fn normalize(args: &mut Args) {
  let Some(base_href) = &args.base_href else {
    return;
  };
  let trimmed = base_href.trim().to_string();
  if is_empty_root_url(&trimmed) {
    args.base_href = None;
    return;
  }
  let mut bh = trimmed;
  if !bh.ends_with('/') {
    bh.push('/');
  }
  if !bh.starts_with('/') {
    bh.insert(0, '/');
  }
  args.base_href = Some(bh);
}

/// 参数组合的合法性检查，不合法时直接退出（与旧行为保持一致）。
pub fn validate(args: &Args) {
  let missing_path = args
    .serve_path
    .as_deref()
    .is_none_or(|path| path.trim().is_empty());
  if missing_path {
    eprintln!("error: missing serve path\n\nUSAGE:\n    lightstatic [OPTIONS] <PATH>\n\nFor more information try --help\n");
    process::exit(-1);
  }
}

pub fn get_args() -> Args {
  let mut args = Args::parse();
  normalize(&mut args);
  if args.log_dir.is_some() || args.no_color {
    colored::control::set_override(false);
  }
  validate(&args);
  args
}
