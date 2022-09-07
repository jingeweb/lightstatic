use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
  /// directory path to serve
  #[clap(value_parser)]
  path: String,
  /// ip address to bind (default: "0.0.0.0")
  #[clap(
    short = 'H',
    long,
    value_parser,
    value_name = "IP",
    default_value = "0.0.0.0"
  )]
  host: String,
  /// port to listen (default: "8080"). if the specified port is not avaiable, find a free port instead.
  #[clap(short, long, value_parser, default_value_t = 8080)]
  port: u32,
  /// gzip encode response content (default: false)
  #[clap(short, long, value_parser, default_value_t = false)]
  gzip: bool,
  /// open browser window after starting the server (default: false)
  #[clap(short, long, value_parser, default_value_t = false)]
  open: bool,
  /// use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
  #[clap(short = '5', long, value_parser, default_value_t = false)]
  html5: bool,
  /// index file to redirect under html5 mode (default: "index.html")
  #[clap(
    short,
    long,
    value_parser,
    value_name = "FILE",
    default_value = "index.html"
  )]
  index: String,
  /// delay in milliseconds for response (default: "0")
  #[clap(short, long, value_parser, default_value_t = 0)]
  delay: u32,
  /// store(cache) static files into memory (default: false)
  #[clap(short = 's', long, value_parser, default_value_t = false)]
  store_in_memory: bool,
  /// cache files which match regexp forever, if specified.
  #[clap(short = 'r', long, value_parser, value_name = "REGEXP")]
  cache_forever_regexp: Option<String>,
  /// write logs to directory, if specified.
  #[clap(short, long, value_parser, value_name = "DIRECTORY")]
  log_dir: Option<String>,
  /// server base href, useful when under nginx subpath
  #[clap(short, long, value_parser)]
  base_href: Option<String>,
  /// do not print access log
  #[clap(short = 'A', long, value_parser, default_value_t = false)]
  no_access_log: bool,
  /// disable color log
  #[clap(short = 'C', long, value_parser, default_value_t = false)]
  no_color: bool,
}

fn main() {
  let args = Args::parse();

  println!("{}", args.host);
}
