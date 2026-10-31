use std::process::Command;

mod common;

use common::{gunzip, temp_dir, write_file, TestServer};

#[test]
fn range_download_supports_bounded_open_and_suffix_ranges() {
  let root = temp_dir("ranges");
  write_file(&root, "file.bin", b"0123456789");
  let server = TestServer::start(&root, &[]);
  let full = server.get("/file.bin");
  assert_eq!(full.header("accept-ranges"), Some("bytes"));
  for (range, content_range, body) in [
    ("bytes=2-5", "bytes 2-5/10", "2345"),
    ("bytes=6-", "bytes 6-9/10", "6789"),
    ("bytes=-3", "bytes 7-9/10", "789"),
    ("bytes=8-999", "bytes 8-9/10", "89"),
    ("bytes=-999", "bytes 0-9/10", "0123456789"),
    ("bytes=0-0", "bytes 0-0/10", "0"),
  ] {
    let res = server.request("/file.bin", &[("range", range)]);
    assert_eq!(res.status, 206, "{range}");
    assert_eq!(res.header("content-range"), Some(content_range));
    assert_eq!(
      res.header("content-length"),
      Some(body.len().to_string().as_str())
    );
    assert_eq!(res.header("etag"), full.header("etag"));
    assert_eq!(res.text(), body);
  }
  // 拼接两次下载的结果，验证真实的续传场景。
  let mut downloaded = server.request("/file.bin", &[("range", "bytes=0-3")]).body;
  downloaded.extend(server.request("/file.bin", &[("range", "bytes=4-")]).body);
  assert_eq!(downloaded, full.body);
}

#[test]
fn range_download_unsatisfiable_and_empty_files() {
  let root = temp_dir("range-empty");
  write_file(&root, "file.bin", b"0123456789");
  write_file(&root, "empty.bin", b"");
  let server = TestServer::start(&root, &[]);
  for (path, range, expected) in [
    ("/file.bin", "bytes=10-", "bytes */10"),
    ("/file.bin", "bytes=-0", "bytes */10"),
    ("/empty.bin", "bytes=0-", "bytes */0"),
    ("/empty.bin", "bytes=-1", "bytes */0"),
  ] {
    let res = server.request(path, &[("range", range)]);
    assert_eq!(res.status, 416, "{path}: {range}");
    assert_eq!(res.header("content-range"), Some(expected));
    assert!(res.body.is_empty());
  }
  assert_eq!(server.get("/empty.bin").status, 200);
}

#[test]
fn range_download_ignores_malformed_and_multiple_ranges() {
  let root = temp_dir("range-invalid");
  write_file(&root, "file.bin", b"0123456789");
  let server = TestServer::start(&root, &[]);
  for range in [
    "items=0-1",
    "bytes=abc",
    "bytes=5-2",
    "bytes=0-1,4-5",
    "bytes=+1-2",
    "bytes=184467440737095516160-",
  ] {
    let res = server.request("/file.bin", &[("range", range)]);
    assert_eq!(res.status, 200, "{range}");
    assert_eq!(res.header("content-range"), None);
    assert_eq!(res.text(), "0123456789");
  }
  let res = server.request(
    "/file.bin",
    &[("range", "bytes=0-1"), ("range", "bytes=4-5")],
  );
  assert_eq!(res.status, 200);
  assert_eq!(res.text(), "0123456789");
}

#[test]
fn range_download_if_range_and_cache_preconditions() {
  let root = temp_dir("range-validator");
  write_file(&root, "file.bin", b"0123456789");
  let server = TestServer::start(&root, &[]);
  let full = server.get("/file.bin");
  let etag = full.header("etag").unwrap();
  let res = server.request("/file.bin", &[("range", "bytes=5-"), ("if-range", etag)]);
  assert_eq!(res.status, 206);
  assert_eq!(res.text(), "56789");
  for validator in [
    "\"stale\"".to_string(),
    format!("W/{etag}"),
    "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
  ] {
    let res = server.request(
      "/file.bin",
      &[("range", "bytes=999-"), ("if-range", &validator)],
    );
    assert_eq!(res.status, 200);
    assert_eq!(res.body, full.body);
  }
  let res = server.request(
    "/file.bin",
    &[("range", "bytes=999-"), ("if-none-match", etag)],
  );
  assert_eq!(res.status, 304);
}

#[test]
fn range_download_gzip_ignores_ranges_but_identity_can_resume() {
  let root = temp_dir("range-gzip");
  write_file(&root, "file.bin", b"0123456789");
  let server = TestServer::start(&root, &["-g"]);
  let identity = server.get("/file.bin");
  for range in ["bytes=2-5", "bytes=999-"] {
    let res = server.request(
      "/file.bin",
      &[("range", range), ("accept-encoding", "gzip")],
    );
    assert_eq!(res.status, 200);
    assert_eq!(res.header("content-encoding"), Some("gzip"));
    assert_eq!(res.header("accept-ranges"), Some("none"));
    assert_eq!(res.header("content-range"), None);
    assert_ne!(res.header("etag"), identity.header("etag"));
    assert_eq!(gunzip(&res.body), identity.body);
    let resumed = server.request(
      "/file.bin",
      &[
        ("range", "bytes=5-"),
        ("if-range", res.header("etag").unwrap()),
      ],
    );
    assert_eq!(resumed.status, 200);
    assert_eq!(resumed.body, identity.body);
  }
  let res = server.request(
    "/file.bin",
    &[("range", "bytes=5-"), ("accept-encoding", "identity")],
  );
  assert_eq!(res.status, 206);
  assert_eq!(res.header("content-encoding"), None);
  assert_eq!(res.text(), "56789");
}

#[test]
fn range_download_head_ignores_range_and_index_routes_support_it() {
  let root = temp_dir("range-routing");
  write_file(&root, "index.html", b"0123456789");
  let server = TestServer::start(&root, &["-5", "-b", "/app/"]);
  for path in ["/app/index.html", "/app/", "/app/spa-route"] {
    let res = server.request(path, &[("range", "bytes=2-5")]);
    assert_eq!(res.status, 206);
    assert_eq!(res.text(), "2345");
    let head =
      common::http_request_with_method(server.port, "HEAD", path, &[("range", "bytes=999-")]);
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-range"), None);
    assert_eq!(head.header("content-length"), Some("10"));
    assert!(head.body.is_empty());
  }
}

#[cfg(unix)]
#[test]
fn regression_html5_fallback_blocks_external_symlink() {
  let root = temp_dir("fallback-symlink");
  let outside = temp_dir("fallback-outside").join("secret.txt");
  std::fs::write(&outside, b"outside-secret").unwrap();
  std::os::unix::fs::symlink(&outside, root.join("index.html")).unwrap();
  let server = TestServer::start(&root, &["-5"]);
  assert_eq!(server.get("/index.html").status, 403);
  let res = server.get("/missing-route");
  assert_eq!(res.status, 403);
  assert!(!res.text().contains("outside-secret"));
}

#[test]
fn regression_html5_fallback_requires_regular_file() {
  let root = temp_dir("fallback-directory");
  std::fs::create_dir(root.join("index.html")).unwrap();
  let server = TestServer::start(&root, &["-5"]);
  assert_eq!(server.get("/missing-route").status, 404);
}

#[test]
fn regression_etag_changes_with_subsecond_modifications() {
  use std::time::{Duration, UNIX_EPOCH};

  let root = temp_dir("etag-subsecond");
  let path = write_file(&root, "asset.txt", b"old");
  let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
  let first = UNIX_EPOCH + Duration::new(1_700_000_000, 100_000_000);
  let second = UNIX_EPOCH + Duration::new(1_700_000_000, 900_000_000);
  file.set_modified(first).unwrap();
  let server = TestServer::start(&root, &[]);
  let before = server.get("/asset.txt");
  let etag = before.header("etag").unwrap();
  std::fs::write(&path, b"new").unwrap();
  file.set_modified(second).unwrap();
  let after = server.request("/asset.txt", &[("if-none-match", etag)]);
  assert_eq!(after.status, 200);
  assert_eq!(after.text(), "new");
  assert_ne!(after.header("etag"), Some(etag));
  let cached = server.request(
    "/asset.txt",
    &[("if-none-match", after.header("etag").unwrap())],
  );
  assert_eq!(cached.status, 304);
}

#[test]
fn regression_directory_redirect_keeps_encoding_query_and_base() {
  let root = temp_dir("directory-redirect");
  write_file(&root, "index.html", b"root");
  write_file(&root, "sub dir/index.html", b"sub");
  for extra in [vec![], vec!["--list-dir"], vec!["-b", "/app/"]] {
    let server = TestServer::start(&root, &extra);
    let base = if extra.contains(&"-b") { "/app" } else { "" };
    let res = server.get(&format!("{base}/sub%20dir?v=1"));
    assert_eq!(res.status, 308);
    let target = format!("{base}/sub%20dir/?v=1");
    assert_eq!(res.header("location"), Some(target.as_str()));
    assert_eq!(server.get(&target).status, 200);
    if !base.is_empty() {
      let res = server.get("/app?v=1");
      assert_eq!(res.status, 308);
      assert_eq!(res.header("location"), Some("/app/?v=1"));
    }
  }
}

#[test]
fn regression_gzip_explicit_refusal_overrides_wildcard() {
  let root = temp_dir("gzip-refusal");
  write_file(&root, "asset.txt", b"plain");
  let server = TestServer::start(&root, &["-g"]);
  let res = server.request("/asset.txt", &[("accept-encoding", "gzip;q=0, *;q=1")]);
  assert_eq!(res.status, 200);
  assert_eq!(res.header("content-encoding"), None);
  assert_eq!(res.text(), "plain");
}

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

  // `//` 指向 root 目录 → 默认返回 root/index.html，而不是文件系统根目录
  let res = server.get("//");
  assert_eq!(res.status, 200);
  assert_eq!(res.text(), "home");
  assert!(!res.text().contains("/usr"));
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

  // 目录默认返回 index.html
  assert_eq!(server.get("/app/").text(), "idx");
}

/// `--list-dir` 下目录仍然列出，且链接带 base href 前缀。
#[test]
fn base_href_listing_keeps_prefix() {
  let root = temp_dir("e2e-base-list");
  write_file(&root, "index.html", b"idx");
  write_file(&root, "x.txt", b"x");

  let server = TestServer::start(&root, &["-b", "/app/", "--list-dir"]);
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

/// 目录默认返回自己的 index.html（不再区分 html5 模式）。
#[test]
fn directory_serves_its_index() {
  let root = temp_dir("e2e-dir-index");
  write_file(&root, "index.html", b"root-index");
  write_file(&root, "sub/index.html", b"sub-index");
  write_file(&root, "plain/a.txt", b"a");

  let server = TestServer::start(&root, &[]);
  assert_eq!(server.get("/").text(), "root-index");
  assert_eq!(server.get("/sub/").text(), "sub-index");

  // 没有 index.html 的目录退回列目录
  let res = server.get("/plain/");
  assert_eq!(res.status, 200);
  assert!(res.text().contains("a.txt"), "{}", res.text());
}

/// `--list-dir`：即使目录里有 index.html 也直接列目录。
#[test]
fn list_dir_flag_forces_listing() {
  let root = temp_dir("e2e-list-dir");
  write_file(&root, "index.html", b"root-index");
  write_file(&root, "sub/index.html", b"sub-index");

  let server = TestServer::start(&root, &["--list-dir"]);
  assert!(server.get("/").text().contains("Index of /"));
  assert!(server.get("/sub/").text().contains("Index of /sub/"));
}

/// `-i/--index` 已移除。
#[test]
fn index_flag_is_gone() {
  for flag in ["-i", "--index"] {
    let out = Command::new(env!("CARGO_BIN_EXE_lightstatic"))
      .arg(".")
      .arg(flag)
      .arg("app.html")
      .output()
      .unwrap();
    assert!(!out.status.success(), "{flag} should be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unexpected argument"), "{flag}: {stderr}");
  }
}
