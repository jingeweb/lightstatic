use async_compression::futures::bufread::GzipEncoder;
use async_std::{fs, io::BufReader, path::Path, stream::StreamExt};
use http_types::{mime::*, Mime};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use tide::{Body, Response};

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
pub async fn send_dir(dir: &Path, prefix: &str) -> tide::Result<Response> {
  let mut list = dir.read_dir().await?;
  let mut items: Vec<(String, bool)> = Vec::new();

  while let Some(item) = list.next().await {
    let item = item?;
    let path = item.path();
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
      continue;
    };
    // metadata() 跟随符号链接，仅用于展示；访问时会再做 root 包含性校验。
    let is_dir = std::fs::metadata(&path)
      .map(|m| m.is_dir())
      .unwrap_or(false);
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
  Ok(
    Response::builder(200)
      .content_type("text/html; charset=utf-8")
      .body(cnt)
      .build(),
  )
}

pub fn get_mime(file: &Path) -> Mime {
  get_mime_from_ext(file.extension().and_then(|p| p.to_str()))
}

pub fn get_mime_from_ext(ext: Option<&str>) -> Mime {
  let Some(ext) = ext else {
    return BYTE_STREAM;
  };
  match ext {
    "html" => HTML,
    "js" | "mjs" | "jsonp" | "ts" | "tsx" | "jsx" => JAVASCRIPT,
    "css" => CSS,
    "jpeg" | "jpg" => JPEG,
    "icon" | "ico" => ICO,
    "png" => PNG,
    "wasm" => WASM,
    "json" => JSON,
    "svg" => SVG,
    "xml" => XML,
    "txt" => PLAIN,
    _ => BYTE_STREAM,
  }
}

/// ETag 按 RFC 7232 需要是带双引号的 quoted-string。
pub fn etag_for(mtime: &str) -> String {
  format!("\"{mtime}\"")
}

/// `Accept-Encoding` 是否接受 gzip（含 `*`，并处理 `q=0`）。
pub fn accepts_gzip(values: Option<&http_types::headers::HeaderValues>) -> bool {
  let Some(values) = values else {
    return false;
  };
  values.iter().any(|value| {
    value.as_str().split(',').any(|part| {
      let mut params = part.split(';');
      let token = params.next().unwrap_or("").trim();
      if !(token.eq_ignore_ascii_case("gzip") || token == "*") {
        return false;
      }
      // q=0 / q=0.0 表示客户端明确拒绝
      !params.any(|param| {
        let param = param.trim().replace(' ', "").to_ascii_lowercase();
        match param.strip_prefix("q=") {
          Some(q) => q.parse::<f32>().map(|q| q == 0.0).unwrap_or(false),
          None => false,
        }
      })
    })
  })
}

pub async fn send_file(
  filepath: &Path,
  stat: &std::fs::Metadata,
  etag: &str,
  gzip: bool,
) -> tide::Result<Response> {
  let file = fs::File::open(filepath).await?;
  let mut body;
  let mut res = Response::builder(200)
    .header("etag", etag)
    .header("vary", "accept-encoding");
  if gzip {
    res = res.header("content-encoding", "gzip");
    body = Body::from_reader(BufReader::new(GzipEncoder::new(BufReader::new(file))), None);
  } else {
    body = Body::from_reader(BufReader::new(file), Some(stat.len() as usize));
  }
  body.set_mime(get_mime(filepath));
  Ok(res.body(body).build())
}

/// 返回 true 表示需要发送内容；false 表示命中 304。
pub fn should_send_file(values: Option<&http_types::headers::HeaderValues>, etag: &str) -> bool {
  let Some(values) = values else {
    return true;
  };
  let target = etag.trim_matches('"');
  for value in values.iter() {
    for candidate in value.as_str().split(',') {
      let candidate = candidate.trim();
      let candidate = candidate.strip_prefix("W/").unwrap_or(candidate);
      if candidate.trim_matches('"') == target {
        return false;
      }
    }
  }
  true
}
