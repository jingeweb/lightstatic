use clap::Parser;

use lightstatic::args::{normalize, Args};
use lightstatic::logger::{initialize_log, log_access, log_startup_info};

fn read_log(dir: &std::path::Path, prefix: &str) -> String {
  let mut found = String::new();
  for entry in std::fs::read_dir(dir).unwrap() {
    let path = entry.unwrap().path();
    if path
      .file_name()
      .and_then(|name| name.to_str())
      .is_some_and(|name| name.starts_with(prefix))
    {
      found.push_str(&std::fs::read_to_string(&path).unwrap());
    }
  }
  found
}

/// `--log-dir` 下 info / access 分文件写入（fern 的 date-based 目的地）。
#[test]
fn writes_logs_into_directory() {
  colored::control::set_override(false);

  let dir = std::env::temp_dir().join(format!("lightstatic-log-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();

  let mut args = Args::parse_from(["lightstatic", "web", "-l", dir.to_str().unwrap()]);
  normalize(&mut args);
  let cwd = std::path::PathBuf::from("/tmp");

  initialize_log(&args, &cwd);
  log_startup_info(&args, 8123);
  log_access(200, "/a.txt");
  log_access(404, "/missing");

  std::thread::sleep(std::time::Duration::from_millis(200));

  let info = read_log(&dir, "info.");
  let access = read_log(&dir, "access.");
  assert!(info.contains("Starting up lightstatic"), "{info}");
  assert!(info.contains("http://127.0.0.1:8123"), "{info}");
  assert!(access.contains("GET 200 /a.txt"), "{access}");
  assert!(access.contains("GET 404 /missing"), "{access}");
}
