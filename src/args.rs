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
  /// index file to redirect under html5 mode (default: "index.html")
  #[clap(
    short,
    long,
    value_parser,
    value_name = "FILE",
    default_value = "index.html"
  )]
  pub index: String,
  /// delay in milliseconds for response (default: "0")
  #[clap(short, long, value_parser, default_value_t = 0)]
  pub delay: u64,
  /// store(cache) static files into memory (default: false)
  #[clap(short, long, value_parser, default_value_t = false)]
  pub cache_in_memory: bool,
  /// cache files which match regexp forever, if specified.
  #[clap(short, long, value_parser, value_name = "REGEXP")]
  pub regex_immutable: Option<regex::Regex>,
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
  /// send signal to running process, action can be "stop" or "refresh"
  #[clap(short, long, value_name = "ACTION")]
  pub signal: Option<String>,
}

pub fn get_args() -> Args {
  let mut args = Args::parse();
  if args.log_dir.is_some() || args.no_color {
    colored::control::set_override(false);
  }
  if args.regex_immutable.is_some() && !args.cache_in_memory {
    eprintln!("--regex-immutable only effect with --cache-in-memory");
    process::exit(-1);
  }
  if args.signal.is_none() && args.serve_path.is_none() {
    eprintln!("error: missing serve path\n\nUSAGE:\n    lightstatic [OPTIONS] <PATH>\n\nFor more information try --help\n");
    process::exit(-1);
  }

  if let Some(base_href) = &mut args.base_href {
    if is_empty_root_url(base_href.trim()) {
      args.base_href.take();
    } else {
      if !base_href.ends_with('/') {
        base_href.push('/');
      }
      if !base_href.starts_with('/') {
        let mut bh = "/".to_string();
        bh.push_str(base_href);
        args.base_href.replace(bh);
      }
    }
  }

  args
}
