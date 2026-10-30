use std::path::{Path, PathBuf};

use lightstatic::util::{is_empty_root_url, resolve_path};

fn path(value: &str) -> PathBuf {
  PathBuf::from(value)
}

fn home() -> PathBuf {
  std::env::var("HOME")
    .ok()
    .map(std::path::PathBuf::from)
    .or_else(dirs::home_dir)
    .expect("home directory")
}

#[test]
fn keeps_absolute_paths() {
  assert_eq!(
    path("/a/b/c"),
    resolve_path(Path::new("/pa/pb"), Path::new("/a/b/c"))
  );
}

#[test]
fn joins_relative_paths() {
  let base = Path::new("/pa/pb/pc/");
  assert_eq!(
    path("/pa/pb/pc/a/b/c"),
    resolve_path(base, Path::new("a/b/c"))
  );
  assert_eq!(
    path("/pa/pb/pc/a/b/c"),
    resolve_path(base, Path::new("./a/b/c"))
  );
}

#[test]
fn collapses_parent_segments() {
  let base = Path::new("/pa/pb/pc/");
  assert_eq!(
    path("/pa/pb/a/b/c"),
    resolve_path(base, Path::new("../a/b/c"))
  );
  // 超出根目录时夹在 '/' 上，不会 panic
  assert_eq!(
    path("/a/b/c"),
    resolve_path(base, Path::new("../../../a/b/c"))
  );
  assert_eq!(
    path("/a/b/c"),
    resolve_path(base, Path::new("../../../../a/b/c"))
  );
}

#[test]
fn expands_home_prefix() {
  assert_eq!(
    home().join("a/b"),
    resolve_path(Path::new("/base"), Path::new("~/a/b"))
  );
  assert_eq!(home(), resolve_path(Path::new("/base"), Path::new("~")));
}

/// 回归：空路径曾导致 `as_bytes()[0]` 下标越界 panic。
#[test]
fn empty_path_does_not_panic() {
  assert_eq!(
    path("/pa/pb"),
    resolve_path(Path::new("/pa/pb"), Path::new(""))
  );
}

/// 回归：非 UTF-8 路径曾触发 `to_str().unwrap()` panic。
#[cfg(unix)]
#[test]
fn non_utf8_path_does_not_panic() {
  use std::os::unix::ffi::OsStrExt;
  let raw = std::ffi::OsStr::from_bytes(b"~\xff\xfe");
  let resolved = resolve_path(Path::new("/base"), Path::new(raw));
  assert!(resolved
    .to_string_lossy()
    .starts_with(&*home().to_string_lossy()));
}

#[test]
fn empty_root_url_detection() {
  assert!(is_empty_root_url(""));
  assert!(is_empty_root_url("/"));
  assert!(!is_empty_root_url("/app"));
  assert!(!is_empty_root_url("/app/"));
  assert!(!is_empty_root_url("x"));
}

#[tokio::test]
async fn detects_paths_inside_root() {
  // is_inside_root 期望传入已规范化的根目录（macOS 上 /var -> /private/var）
  let root = std::fs::canonicalize(temp_root()).unwrap();
  let inside = root.join("a.txt");
  std::fs::write(&inside, b"x").unwrap();
  assert!(lightstatic::util::is_inside_root(&root, Path::new(&inside)).await);

  let outside = std::env::temp_dir().join("lightstatic-outside.txt");
  std::fs::write(&outside, b"x").unwrap();
  assert!(!lightstatic::util::is_inside_root(&root, Path::new(&outside)).await);
  let _ = std::fs::remove_file(&outside);
}

fn temp_root() -> PathBuf {
  let dir = std::env::temp_dir().join(format!("lightstatic-util-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}
