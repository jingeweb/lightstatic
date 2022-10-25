use crate::util::resolve_path;
use crate::{args, store::FileCacheStore};
use async_std::path::{Path, PathBuf};
use async_std::sync::RwLock;

pub struct Config {
  pub delay: u64,
  pub should_log_access: bool,
  pub root_dir: PathBuf,
  pub base_href: Option<String>,
  pub html5: bool,
  pub index: String,
  pub index_href: PathBuf,
  pub gzip: bool,
  pub cache_store: Option<RwLock<FileCacheStore>>,
}
impl Config {
  pub fn new(args: &args::Args, cwd: &Path) -> Self {
    let root_dir = resolve_path(cwd, Path::new(args.serve_path.as_ref().unwrap()));
    Config {
      delay: args.delay,
      should_log_access: !args.no_access,
      base_href: args.base_href.clone(),
      html5: args.html5,
      index_href: root_dir.join(&args.index),
      index: args.index.clone(),
      root_dir,
      gzip: args.gzip,
      cache_store: None,
    }
  }
}

pub type AppConfig = std::sync::Arc<Config>;
