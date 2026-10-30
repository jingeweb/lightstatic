use clap::Parser;

use lightstatic::args::{normalize, Args};

fn parse(argv: &[&str]) -> Args {
  let mut args = Args::parse_from(argv);
  normalize(&mut args);
  args
}

#[test]
fn base_href_gets_normalized() {
  assert_eq!(
    parse(&["lightstatic", "dir", "-b", "app"])
      .base_href
      .as_deref(),
    Some("/app/")
  );
  assert_eq!(
    parse(&["lightstatic", "dir", "-b", "/app"])
      .base_href
      .as_deref(),
    Some("/app/")
  );
  assert_eq!(
    parse(&["lightstatic", "dir", "-b", "/app/"])
      .base_href
      .as_deref(),
    Some("/app/")
  );
  assert_eq!(
    parse(&["lightstatic", "dir", "-b", "  /x  "])
      .base_href
      .as_deref(),
    Some("/x/")
  );
}

#[test]
fn empty_base_href_is_dropped() {
  assert_eq!(parse(&["lightstatic", "dir", "-b", "/"]).base_href, None);
  assert_eq!(parse(&["lightstatic", "dir", "-b", ""]).base_href, None);
  assert_eq!(parse(&["lightstatic", "dir", "-b", "   "]).base_href, None);
  assert_eq!(parse(&["lightstatic", "dir"]).base_href, None);
}

#[test]
fn defaults_are_stable() {
  let args = parse(&["lightstatic", "dir"]);
  assert_eq!(args.host, "0.0.0.0");
  assert_eq!(args.port, 8080);
  assert_eq!(args.delay, 0);
  assert!(!args.gzip);
  assert!(!args.html5);
  assert!(!args.list_dir);
  assert_eq!(args.serve_path.as_deref(), Some("dir"));
}

/// 内存缓存相关选项已整体移除，不应再被接受。
#[test]
fn cache_flags_are_gone() {
  assert!(Args::try_parse_from(["lightstatic", "dist", "-c"]).is_err());
  assert!(Args::try_parse_from(["lightstatic", "dist", "--cache-in-memory"]).is_err());
  assert!(Args::try_parse_from(["lightstatic", "dist", "-r", r"\.css$"]).is_err());
  assert!(Args::try_parse_from(["lightstatic", "dist", "--regex-immutable", r"\.css$"]).is_err());
}

/// `--signal` 已整体移除，不应再被接受。
#[test]
fn signal_flag_is_gone() {
  assert!(Args::try_parse_from(["lightstatic", "dir", "--signal", "stop"]).is_err());
  assert!(Args::try_parse_from(["lightstatic", "dir", "-s", "stop"]).is_err());
}

#[test]
fn flags_are_parsed() {
  let args = parse(&[
    "lightstatic",
    "dist",
    "-5",
    "-g",
    "-A",
    "-p",
    "9000",
    "-d",
    "250",
    "--list-dir",
  ]);
  assert!(args.html5);
  assert!(args.gzip);
  assert!(args.no_access);
  assert!(args.list_dir);
  assert_eq!(args.port, 9000);
  assert_eq!(args.delay, 250);
}

/// `-i/--index` 已移除：目录一律用 index.html，除非传 `--list-dir`。
#[test]
fn index_flag_is_gone() {
  assert!(Args::try_parse_from(["lightstatic", "dist", "-i", "app.html"]).is_err());
  assert!(Args::try_parse_from(["lightstatic", "dist", "--index", "app.html"]).is_err());
}
