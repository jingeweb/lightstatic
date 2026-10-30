use std::process::Command;

mod common;

use common::{gunzip, temp_dir, write_file, TestServer};

/// 回归：`GET //etc/hosts` 曾经读到根目录之外的文件。
#[test]
fn double_slash_traversal_is_blocked() {
  let root = temp_dir("e2e-traversal");
  write_file(&root, "index.html", b"home");
  write_file(&root, "etc/hosts", b"local-hosts");

  let server = TestServer::start(&root, &[]);

  // 前导斜杠被折叠成相对路径，命中的是 root 里的同名文件
  let res = server.get("//etc/hosts");
  assert_eq!(res.status, 200);
  assert_eq!(res.text(), "local-hosts");

  // root 里没有的文件仍然是 404，而不是读到系统文件
  assert_eq!(server.get("//etc/passwd").status, 404);

  // `//` 只列出 root，不能列出文件系统根目录
  let res = server.get("//");
  assert_eq!(res.status, 200);
  let body = res.text();
  assert!(body.contains("index.html"), "{body}");
  assert!(!body.contains("/usr"), "{body}");
}

/// 回归：请求路径从不做 percent-decode，带空格/中文的文件名恒 404。
#[test]
fn percent_encoded_names_are_served() {
  let root = temp_dir("e2e-encoded");
  write_file(&root, "a b.txt", b"spaced");
  write_file(&root, "中文.txt", b"chinese");

  let server = TestServer::start(&root, &[]);
  assert_eq!(server.get("/a%20b.txt").text(), "spaced");
  assert_eq!(server.get("/%E4%B8%AD%E6%96%87.txt").text(), "chinese");

  // 列表里的链接也必须编码，否则点不开
  let body = server.get("/").text();
  assert!(body.contains("href=\"/a%20b.txt\""), "{body}");
}

/// 回归：目录列表把文件名原样插进 HTML（XSS）。
/// Windows 文件名不允许 `<>`，该用例只在 unix 上跑。
#[cfg(unix)]
#[test]
fn directory_listing_escapes_html() {
  let root = temp_dir("e2e-xss");
  write_file(&root, "<img src=x onerror=alert(1)>.txt", b"x");

  let server = TestServer::start(&root, &[]);
  let body = server.get("/").text();
  assert!(!body.contains("<img src=x"), "{body}");
  assert!(body.contains("&lt;img"), "{body}");
}

/// 回归：`-g` 时无视 Accept-Encoding，永远返回 gzip。
#[test]
fn gzip_negotiation() {
  let root = temp_dir("e2e-gzip");
  let big = "x".repeat(50_000);
  write_file(&root, "big.txt", big.as_bytes());

  let server = TestServer::start(&root, &["-g"]);

  let res = server.request("/big.txt", &[("accept-encoding", "gzip")]);
  assert_eq!(res.status, 200);
  assert_eq!(res.header("content-encoding"), Some("gzip"));
  assert_eq!(gunzip(&res.body), big.as_bytes());

  let res = server.request("/big.txt", &[("accept-encoding", "identity")]);
  assert_eq!(res.header("content-encoding"), None);
  assert_eq!(res.body, big.as_bytes());
}

/// 回归：html5 回退只看“URL 里有没有点”，`/user/1.0/profile` 会 404。
#[test]
fn html5_fallback_ignores_dots_in_parent_segments() {
  let root = temp_dir("e2e-html5");
  write_file(&root, "index.html", b"spa");

  let server = TestServer::start(&root, &["-5"]);
  assert_eq!(server.get("/deep/route").text(), "spa");

  let res = server.get("/user/1.0/profile");
  assert_eq!(res.status, 200);
  assert_eq!(res.text(), "spa");

  assert_eq!(server.get("/v1.2/").status, 200);
  // 真正像静态文件的请求仍然 404
  assert_eq!(server.get("/missing.js").status, 404);
}

#[test]
fn base_href_redirects_and_scopes() {
  let root = temp_dir("e2e-base");
  write_file(&root, "index.html", b"idx");
  write_file(&root, "x.txt", b"x");

  let server = TestServer::start(&root, &["-b", "/app/"]);

  let res = server.get("/");
  assert_eq!(res.status, 302);
  assert_eq!(res.header("location"), Some("/app/"));

  assert_eq!(server.get("/app/x.txt").text(), "x");
  assert_eq!(server.get("/other/x.txt").status, 403);
  // 前缀下面也不能用双斜杠逃逸
  assert_eq!(server.get("/app//etc/passwd").status, 404);

  let body = server.get("/app/").text();
  assert!(body.contains("href=\"/app/x.txt\""), "{body}");
}

#[test]
fn dotdot_and_bad_encoding_are_rejected() {
  let root = temp_dir("e2e-dotdot");
  write_file(&root, "index.html", b"idx");

  let server = TestServer::start(&root, &[]);

  // `..` 可能被 url 层归一化掉，也可能落到 sanitize_path；两种都不允许读到 root 外的文件
  for path in [
    "/../etc/passwd",
    "/%2e%2e/etc/passwd",
    "/a/../../etc/passwd",
  ] {
    let res = server.get(path);
    assert!(
      res.status == 403 || res.status == 404,
      "{path} -> {}",
      res.status
    );
    assert!(!res.text().contains("root:"), "{path} leaked /etc/passwd");
  }

  assert_eq!(server.get("/%FF").status, 400);
}

#[test]
fn etag_enables_304() {
  let root = temp_dir("e2e-etag");
  write_file(&root, "a.txt", b"a");

  let server = TestServer::start(&root, &[]);
  let res = server.get("/a.txt");
  let etag = res.header("etag").expect("etag header").to_string();
  assert!(etag.starts_with('"'), "{etag}");

  let res = server.request("/a.txt", &[("if-none-match", &etag)]);
  assert_eq!(res.status, 304);
  assert_eq!(res.header("etag"), Some(etag.as_str()));
}

#[cfg(unix)]
#[test]
fn symlink_outside_root_is_forbidden() {
  let root = temp_dir("e2e-symlink");
  let outside = temp_dir("e2e-symlink-outside").join("secret.txt");
  std::fs::write(&outside, b"secret").unwrap();
  std::os::unix::fs::symlink(&outside, root.join("leak.txt")).unwrap();

  let server = TestServer::start(&root, &[]);
  let res = server.get("/leak.txt");
  assert_eq!(res.status, 403);
  assert_ne!(res.text(), "secret");
}

/// 回归：`lightstatic ""` 曾经在 util::resolve_path 里下标越界 panic。
#[test]
fn empty_serve_path_reports_error() {
  let out = Command::new(env!("CARGO_BIN_EXE_lightstatic"))
    .arg("")
    .output()
    .unwrap();

  assert!(!out.status.success());
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("missing serve path"), "{stderr}");
  assert!(!stderr.contains("panicked"), "{stderr}");
}
