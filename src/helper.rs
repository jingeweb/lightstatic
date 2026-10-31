use async_compression::tokio::bufread::GzipEncoder;
use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::header::{
  HeaderMap, HeaderValue, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG,
  IF_NONE_MATCH, VARY,
};
use hyper::{Response, StatusCode};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::io;
use std::path::Path;
use tokio::io::BufReader;
use tokio_util::io::ReaderStream;

/// 响应体统一用 boxed body，错误类型固定为 io::Error（读文件/解压都可能失败）。
pub type ResBody = BoxBody<Bytes, io::Error>;

pub fn empty_body() -> ResBody {
  Full::new(Bytes::new())
    .map_err(|never| match never {})
    .boxed()
}

pub fn full_body(data: impl Into<Bytes>) -> ResBody {
  Full::new(data.into())
    .map_err(|never| match never {})
    .boxed()
}

fn stream_body<R>(reader: R) -> ResBody
where
  R: tokio::io::AsyncRead + Send + Sync + 'static,
{
  StreamBody::new(ReaderStream::new(reader).map_ok(Frame::data)).boxed()
}

/// 只有状态码、没有响应体的响应。
pub fn response(status: StatusCode) -> Response<ResBody> {
  let mut res = Response::new(empty_body());
  *res.status_mut() = status;
  res
}

/// 目录列表里的文件名/请求路径要按 HTML 文本转义，否则形如
/// `<img src=x onerror=...>.txt` 的文件名会变成可执行的 HTML。
pub fn escape_html(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#x27;"),
      _ => out.push(c),
    }
  }
  out
}

/// `href` 里的路径要做百分号编码（保留 `/` 作为分隔符）。
pub fn encode_path(path: &str) -> String {
  const SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'\\')
    .add(b'^')
    .add(b'|')
    .add(b'[')
    .add(b']');
  utf8_percent_encode(path, SET).to_string()
}

/// 目录列表。`prefix` 是形如 `/`、`/sub/`、`/app/sub/` 的 URL 前缀（未编码）。
pub async fn send_dir(dir: &Path, prefix: &str) -> io::Result<Response<ResBody>> {
  let mut list = tokio::fs::read_dir(dir).await?;
  let mut items: Vec<(String, bool)> = Vec::new();

  while let Some(entry) = list.next_entry().await? {
    let path = entry.path();
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
      continue;
    };
    // metadata() 跟随符号链接，仅用于展示；访问时会再做 root 包含性校验。
    let is_dir = entry.metadata().await.map(|m| m.is_dir()).unwrap_or(false);
    items.push((name.to_string(), is_dir));
  }
  items.sort_by(|a, b| a.0.cmp(&b.0));

  let mut cnt = String::new();
  for (name, is_dir) in &items {
    let href = encode_path(&format!("{prefix}{name}"));
    let href = if *is_dir { format!("{href}/") } else { href };
    cnt.push_str(&format!(
      "  <li><a href=\"{}\">{}</a></li>\n",
      href,
      escape_html(name)
    ));
  }

  let title = escape_html(prefix);
  let cnt = format!(
    "<!DOCTYPE html>
<html>
<head>
<meta charset=\"utf-8\"/>
<title>Index of {0}</title>
<style>
li {{ padding: 6px; }}
</style>
</head>
<body>
<h1>Index of {0}</h1>
<ul>
{1}
</ul>
</body>
</html>",
    title, cnt
  );

  let mut res = response(StatusCode::OK);
  res.headers_mut().insert(
    CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
  );
  *res.body_mut() = full_body(cnt);
  Ok(res)
}

pub fn mime_for(path: &Path) -> &'static str {
  mime_from_ext(path.extension().and_then(|p| p.to_str()))
}

pub fn mime_from_ext(ext: Option<&str>) -> &'static str {
  match ext {
    Some("html") => "text/html; charset=utf-8",
    Some("js") | Some("mjs") | Some("jsonp") | Some("ts") | Some("tsx") | Some("jsx") => {
      "application/javascript; charset=utf-8"
    }
    Some("css") => "text/css; charset=utf-8",
    Some("jpeg") | Some("jpg") => "image/jpeg",
    Some("icon") | Some("ico") => "image/x-icon",
    Some("png") => "image/png",
    Some("wasm") => "application/wasm",
    Some("json") => "application/json",
    Some("svg") => "image/svg+xml",
    Some("xml") => "application/xml",
    Some("txt") => "text/plain; charset=utf-8",
    _ => "application/octet-stream",
  }
}

/// ETag 按 RFC 7232 需要是带双引号的 quoted-string。
pub fn etag_for(mtime: &str) -> String {
  format!("\"{mtime}\"")
}

/// `Accept-Encoding` 是否接受 gzip（含 `*`，并处理 `q=0`）。
pub fn accepts_gzip(headers: &HeaderMap) -> bool {
  let mut gzip = None;
  let mut wildcard = None;
  for value in headers.get_all(ACCEPT_ENCODING) {
    let Ok(value) = value.to_str() else {
      continue;
    };
    for part in value.split(',') {
      let mut params = part.split(';');
      let token = params.next().unwrap_or("").trim();
      let preference = if token.eq_ignore_ascii_case("gzip") {
        &mut gzip
      } else if token == "*" {
        &mut wildcard
      } else {
        continue;
      };
      let mut quality = 1.0_f32;
      for param in params {
        if let Some((name, value)) = param.split_once('=') {
          if name.trim().eq_ignore_ascii_case("q") {
            quality = value.trim().parse().unwrap_or(0.0);
          }
        }
      }
      *preference = Some(quality > 0.0 && quality <= 1.0);
    }
  }
  // 明确列出的 gzip 优先级高于通配符，包括显式的 q=0。
  gzip.or(wildcard).unwrap_or(false)
}

pub async fn send_file(
  filepath: &Path,
  len: u64,
  etag: &str,
  gzip: bool,
) -> io::Result<Response<ResBody>> {
  let file = tokio::fs::File::open(filepath).await?;
  let mut res = response(StatusCode::OK);
  {
    let headers = res.headers_mut();
    headers.insert(
      ETAG,
      HeaderValue::from_str(etag).unwrap_or_else(|_| HeaderValue::from_static("\"\"")),
    );
    headers.insert(VARY, HeaderValue::from_static("accept-encoding"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(mime_for(filepath)));
  }

  if gzip {
    res
      .headers_mut()
      .insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    *res.body_mut() = stream_body(GzipEncoder::new(BufReader::new(file)));
  } else {
    res
      .headers_mut()
      .insert(CONTENT_LENGTH, HeaderValue::from(len));
    *res.body_mut() = stream_body(file);
  }
  Ok(res)
}

/// 返回 true 表示需要发送内容；false 表示命中 304。
pub fn should_send_file(headers: &HeaderMap, etag: &str) -> bool {
  let Some(value) = headers.get(IF_NONE_MATCH) else {
    return true;
  };
  let Ok(value) = value.to_str() else {
    return true;
  };
  let target = etag.trim_matches('"');
  for candidate in value.split(',') {
    let candidate = candidate.trim();
    let candidate = candidate.strip_prefix("W/").unwrap_or(candidate);
    if candidate.trim_matches('"') == target {
      return false;
    }
  }
  true
}
