use crate::args;
use crate::util::resolve_path;
use async_std::path::{Path, PathBuf};

/// 目录请求默认会尝试这个文件（除非传了 `--list-dir`）。
pub const INDEX_FILE: &str = "index.html";

pub struct Config {
  pub delay: u64,
  pub should_log_access: bool,
  pub root_dir: PathBuf,
  /// `root_dir` 的规范化绝对路径，用于阻断符号链接逃逸。
  pub root_canonical: PathBuf,
  pub base_href: Option<String>,
  pub html5: bool,
  pub index_href: PathBuf,
  pub list_dir: bool,
  pub gzip: bool,
}

impl Config {
  pub fn new(args: &args::Args, cwd: &Path) -> Self {
    let root_dir = resolve_path(cwd, Path::new(args.serve_path.as_ref().unwrap()));
    let root_canonical = std::fs::canonicalize(root_dir.as_os_str())
      .map(PathBuf::from)
      .unwrap_or_else(|_| root_dir.clone());
    Config {
      delay: args.delay,
      should_log_access: !args.no_access,
      base_href: args.base_href.clone(),
      html5: args.html5,
      index_href: root_dir.join(INDEX_FILE),
      list_dir: args.list_dir,
      root_dir,
      root_canonical,
      gzip: args.gzip,
    }
  }
}

pub type AppConfig = std::sync::Arc<Config>;
