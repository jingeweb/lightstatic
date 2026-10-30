use std::path::Path;

use http_body_util::BodyExt;
use hyper::header::{HeaderMap, HeaderName, HeaderValue};

use lightstatic::helper::{
  accepts_gzip, encode_path, escape_html, etag_for, mime_for, mime_from_ext, send_dir,
  should_send_file, ResBody,
};

mod common;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
  let mut map = HeaderMap::new();
  for (name, value) in pairs {
    map.insert(
      name.parse::<HeaderName>().unwrap(),
      HeaderValue::from_str(value).unwrap(),
    );
  }
  map
}

fn gzip_ok(value: &str) -> bool {
  accepts_gzip(&headers(&[("accept-encoding", value)]))
}

fn conditional(value: &str, etag: &str) -> bool {
  should_send_file(&headers(&[("if-none-match", value)]), etag)
}

async fn body_of(res: hyper::Response<ResBody>) -> String {
  let bytes = res.into_body().collect().await.unwrap().to_bytes();
  String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn mime_mapping() {
  assert_eq!(mime_from_ext(Some("html")), "text/html; charset=utf-8");
  assert_eq!(mime_from_ext(Some("css")), "text/css; charset=utf-8");
  assert_eq!(
    mime_from_ext(Some("js")),
    "application/javascript; charset=utf-8"
  );
  assert_eq!(
    mime_from_ext(Some("ts")),
    "application/javascript; charset=utf-8"
  );
  assert_eq!(mime_from_ext(Some("json")), "application/json");
  assert_eq!(mime_from_ext(Some("svg")), "image/svg+xml");
  assert_eq!(mime_from_ext(Some("png")), "image/png");
  assert_eq!(mime_from_ext(Some("bin")), "application/octet-stream");
  assert_eq!(mime_from_ext(None), "application/octet-stream");
  assert_eq!(
    mime_for(Path::new("a/b/c.txt")),
    "text/plain; charset=utf-8"
  );
  assert_eq!(mime_for(Path::new("a/b/noext")), "application/octet-stream");
}

#[test]
fn html_escaping() {
  assert_eq!(
    escape_html("<img src=\"x\"> & 'q'"),
    "&lt;img src=&quot;x&quot;&gt; &amp; &#x27;q&#x27;"
  );
  assert_eq!(escape_html("plain"), "plain");
  assert_eq!(escape_html("中文"), "中文");
}

#[test]
fn href_encoding() {
  assert_eq!(encode_path("/a b.txt"), "/a%20b.txt");
  assert_eq!(encode_path("/<img>.txt"), "/%3Cimg%3E.txt");
  assert_eq!(encode_path("/中文.txt"), "/%E4%B8%AD%E6%96%87.txt");
  assert_eq!(encode_path("/a/b/c"), "/a/b/c");
  assert_eq!(encode_path("/q?x#y"), "/q%3Fx%23y");
}

#[test]
fn etag_is_quoted() {
  assert_eq!(etag_for("1710000000"), "\"1710000000\"");
}

#[test]
fn gzip_negotiation() {
  assert!(!accepts_gzip(&HeaderMap::new()));
  assert!(gzip_ok("gzip"));
  assert!(gzip_ok("br, gzip;q=0.8"));
  assert!(gzip_ok("*"));
  assert!(!gzip_ok("identity"));
  assert!(!gzip_ok("br"));
  assert!(!gzip_ok("gzip;q=0"));
  assert!(!gzip_ok("gzip; q=0.0"));
}

#[test]
fn conditional_request_matching() {
  assert!(should_send_file(&HeaderMap::new(), "\"123\""));
  assert!(!conditional("\"123\"", "\"123\""));
  assert!(!conditional("W/\"123\"", "\"123\""));
  assert!(conditional("\"456\"", "\"123\""));
  assert!(!conditional("\"456\", \"123\"", "\"123\""));
}

/// Windows 不允许文件名里出现 `<` / `"`，这条用例只在 unix 上跑。
#[cfg(unix)]
#[tokio::test]
async fn dir_listing_escapes_names_and_encodes_links() {
  let dir = common::temp_dir("helper-dir");
  common::write_file(&dir, "<img src=x onerror=alert(1)>.txt", b"x");
  common::write_file(&dir, "a b.txt", b"x");
  common::write_file(&dir, "quote\"name.txt", b"x");
  std::fs::create_dir(dir.join("sub")).unwrap();

  let res = send_dir(Path::new(&dir), "/").await.unwrap();
  let body = body_of(res).await;

  // 文件名里的标签必须被转义
  assert!(
    body.contains("&lt;img src=x onerror=alert(1)&gt;.txt"),
    "escaped name missing: {body}"
  );
  assert!(!body.contains("<img src=x"), "raw tag leaked: {body}");

  // 链接里的路径必须编码，否则带空格/引号的文件点不开
  assert!(body.contains("href=\"/a%20b.txt\""), "{body}");
  assert!(body.contains("href=\"/quote%22name.txt\""), "{body}");

  // 目录链接带尾斜杠
  assert!(body.contains("href=\"/sub/\""), "{body}");

  // 排序稳定：a b.txt 在 sub/ 之前
  let first = body.find("a%20b.txt").unwrap();
  let second = body.find("sub/").unwrap();
  assert!(first < second, "{body}");
}

#[tokio::test]
async fn dir_listing_prefix_is_applied() {
  let dir = common::temp_dir("helper-prefix");
  common::write_file(&dir, "x.txt", b"x");

  let res = send_dir(Path::new(&dir), "/app/sub/").await.unwrap();
  let body = body_of(res).await;
  assert!(body.contains("href=\"/app/sub/x.txt\""), "{body}");
  assert!(body.contains("Index of /app/sub/"), "{body}");
}
