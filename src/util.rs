use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};

/// 仅适用于 unix/macos 系统的，和 nodejs 的 path.resolve 基本等价的实用函数。
///
/// 注意此函数不是最高效的版本，没有通过 Cow 来避免返回值和入参一致时复用入参（避免分配内存），
/// 并且 dirs::home_dir() 也每次会分配内存，因此不适用于高频调用场景。
pub fn resolve_path(base_path: &Path, file_path: &Path) -> PathBuf {
  if file_path.is_absolute() {
    return file_path.to_path_buf();
  }

  let raw = file_path.to_string_lossy();
  if let Some(rest) = raw.strip_prefix('~') {
    // 此处 dirs.home_dir() 会每次调用都分配内存。
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    return home_dir.join(rest.trim_start_matches(['/', '\\']));
  }

  // path-absolutize 4.x 不再把结果包在 Result 里。
  match file_path.absolutize_from(base_path) {
    std::borrow::Cow::Owned(p) => p,
    std::borrow::Cow::Borrowed(p) => p.to_path_buf(),
  }
}

pub fn is_empty_root_url(p: &str) -> bool {
  if p.len() > 1 {
    return false;
  }
  p.chars().next().map(|c| c == '/').unwrap_or(true)
}

/// 校验 `path` 规范化之后仍然位于 `root_canonical`（已规范化的根目录）之内。
///
/// 用于阻断通过符号链接读到根目录之外的文件：`root/secret -> /etc/passwd`。
pub async fn is_inside_root(root_canonical: &Path, path: &Path) -> bool {
  match tokio::fs::canonicalize(path).await {
    Ok(path) => path.starts_with(root_canonical),
    Err(_) => false,
  }
}
