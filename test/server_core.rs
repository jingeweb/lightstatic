use lightstatic::server_core::{sanitize_path, strip_base_href, url_prefix, PathError};

/// 回归：`//etc/hosts` 曾经被 `&path[1..]` 变成绝对路径，最终读到根目录之外。
#[test]
fn double_slash_cannot_escape_root() {
  assert_eq!(sanitize_path("//etc/hosts").unwrap(), "etc/hosts");
  assert_eq!(sanitize_path("///etc/hosts").unwrap(), "etc/hosts");
  assert_eq!(sanitize_path("//").unwrap(), "");
  assert_eq!(sanitize_path("/").unwrap(), "");
  assert_eq!(sanitize_path("").unwrap(), "");
}

#[test]
fn dotdot_is_rejected() {
  assert_eq!(sanitize_path("/../etc/passwd"), Err(PathError::Forbidden));
  assert_eq!(
    sanitize_path("/a/../../etc/passwd"),
    Err(PathError::Forbidden)
  );
  // 编码后的 .. 也要拦
  assert_eq!(
    sanitize_path("/%2e%2e/etc/passwd"),
    Err(PathError::Forbidden)
  );
  assert_eq!(sanitize_path("/a/%2E%2E/b"), Err(PathError::Forbidden));
}

#[test]
fn percent_encoding_is_decoded() {
  assert_eq!(sanitize_path("/a%20b.txt").unwrap(), "a b.txt");
  assert_eq!(
    sanitize_path("/%E4%B8%AD%E6%96%87.txt").unwrap(),
    "中文.txt"
  );
  assert_eq!(sanitize_path("/a%23b.txt").unwrap(), "a#b.txt");
  assert_eq!(sanitize_path("/%3Ctag%3E").unwrap(), "<tag>");
}

#[test]
fn invalid_encoding_is_bad_request() {
  assert_eq!(sanitize_path("/%FF"), Err(PathError::BadRequest));
  assert_eq!(sanitize_path("/a%00b"), Err(PathError::BadRequest));
}

#[test]
fn dot_segments_are_collapsed() {
  assert_eq!(sanitize_path("/./a/./b").unwrap(), "a/b");
  assert_eq!(sanitize_path("/a//b").unwrap(), "a/b");
}

#[test]
fn base_href_stripping() {
  assert_eq!(strip_base_href("/app/", "/app/"), Some(""));
  assert_eq!(strip_base_href("/app", "/app/"), Some(""));
  assert_eq!(strip_base_href("/app/x", "/app/"), Some("x"));
  assert_eq!(
    strip_base_href("/app//etc/hosts", "/app/"),
    Some("/etc/hosts")
  );
  assert_eq!(strip_base_href("/application", "/app/"), None);
  assert_eq!(strip_base_href("/", "/app/"), None);
}

#[test]
fn url_prefixes() {
  assert_eq!(url_prefix(None, ""), "/");
  assert_eq!(url_prefix(None, "sub"), "/sub/");
  assert_eq!(url_prefix(Some("/app/"), ""), "/app/");
  assert_eq!(url_prefix(Some("/app/"), "sub"), "/app/sub/");
}
