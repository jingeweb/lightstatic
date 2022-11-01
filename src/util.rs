// use std::{os::unix::prelude::OsStrExt};

use std::os::unix::prelude::OsStrExt;

use async_std::path::{Path, PathBuf};
use path_absolutize::Absolutize;

/// 仅适用于 unix/macos 系统的，和 nodejs 的 path.resolve 基本等价的实用函数。
///
/// 注意此函数不是最高效的版本，没有通过 Cow 来避免返回值和入参一致时复用入参（避免分配内存），
/// 并且 dirs::home_dir() 也每次会分配内存，因此不适用于高频调用场景。
pub fn resolve_path(base_path: &Path, file_path: &Path) -> PathBuf {
  let first_char = file_path.as_os_str().as_bytes()[0];
  if first_char == b'/' {
    // println!("return absolute filepath");
    PathBuf::from(file_path)
  } else if first_char == b'~' {
    // 此处 dirs.home_dir() 会每次调用都分配内存。
    let home_dir = PathBuf::from(dirs::home_dir().unwrap());
    let path_str = file_path.to_str().unwrap();
    home_dir.join(&path_str[1..])
  } else {
    type StdPath = std::path::Path;
    let z = StdPath::new(file_path)
      .absolutize_from(StdPath::new(base_path))
      .unwrap();
    match z {
      std::borrow::Cow::Owned(z) => PathBuf::from(z),
      std::borrow::Cow::Borrowed(z) => PathBuf::from(z),
    }
  }
}

pub fn is_empty_root_url(p: &str) -> bool {
  if p.len() > 1 {
    return false;
  }
  p.chars().next().map(|c| c == '/').unwrap_or(true)
}

#[test]
fn test_resolve_path() {
  let home = PathBuf::from(dirs::home_dir().unwrap());
  let cwd = PathBuf::from("/pa/pb/pc/");
  assert_eq!(
    "/a/b/c",
    resolve_path(&cwd, &PathBuf::from("/a/b/c"))
      .to_str()
      .unwrap()
  );
  assert_eq!(
    home.join("a/b/c"),
    resolve_path(&cwd, &PathBuf::from("~a/b/c"))
  );
  assert_eq!(
    cwd.join("a/b/c"),
    resolve_path(&cwd, &PathBuf::from("a/b/c"))
  );
  assert_eq!(
    cwd.join("a/b/c"),
    resolve_path(&cwd, &PathBuf::from("./a/b/c"))
  );
  assert_eq!(
    "/pa/pb/a/b/c",
    resolve_path(&cwd, &PathBuf::from("../a/b/c"))
      .to_str()
      .unwrap()
  );
  assert_eq!(
    "/pa/a/b/c",
    resolve_path(&cwd, &PathBuf::from("../../a/b/c"))
      .to_str()
      .unwrap()
  );
  assert_eq!(
    "/a/b/c",
    resolve_path(&cwd, &PathBuf::from("../../../a/b/c"))
      .to_str()
      .unwrap()
  );
  assert_eq!(
    "/a/b/c",
    resolve_path(&cwd, &PathBuf::from("../../../../a/b/c"))
      .to_str()
      .unwrap()
  );

  // let a = PathBuf::from("/a");
  // {
  //   let mut b = resolve_path(&cwd, &a);
  //   b.push(PathBuf::from("bb"));
  //   println!("{:?}", b);
  // }
  // println!("{:?}", a);
  // panic!("")
}
