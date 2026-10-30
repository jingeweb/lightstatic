use std::path::{Path, PathBuf};
use std::time::Duration;

use hyper::body::Incoming;
use hyper::header::{HeaderValue, ETAG, LOCATION};
use hyper::{Method, Request, Response, StatusCode};
use percent_encoding::percent_decode_str;

use crate::config::{AppConfig, Config, INDEX_FILE};
use crate::helper::{
  accepts_gzip, etag_for, response, send_dir, send_file, should_send_file, ResBody,
};
use crate::logger::log_access;
use crate::util::{is_empty_root_url, is_inside_root};

#[derive(Debug, PartialEq, Eq)]
pub enum PathError {
  /// 无法解码（非法 UTF-8 / 含 NUL）
  BadRequest,
  /// 试图越出根目录（`..`）
  Forbidden,
}

/// 把请求里的 URL 路径（可能带 `%XX`）转换成相对于 root 的安全路径。
///
/// - 解码 percent-encoding：浏览器访问 `a b.txt` / 中文文件名时都会编码
/// - 丢弃空段与 `.`，拒绝 `..`
/// - 结果永远不以 `/` 开头，因此 `root.join(rel)` 不会因为“绝对路径”替换掉 root
pub fn sanitize_path(raw: &str) -> Result<String, PathError> {
  let decoded = percent_decode_str(raw)
    .decode_utf8()
    .map_err(|_| PathError::BadRequest)?;
  if decoded.contains('\0') {
    return Err(PathError::BadRequest);
  }

  let mut parts: Vec<&str> = Vec::new();
  for segment in decoded.split('/') {
    match segment {
      "" | "." => continue,
      ".." => return Err(PathError::Forbidden),
      other => parts.push(other),
    }
  }
  Ok(parts.join("/"))
}

/// 去掉 `--base-href` 前缀，返回仍然带 `%XX` 编码的剩余路径。
pub fn strip_base_href<'a>(path: &'a str, base: &str) -> Option<&'a str> {
  if path == base.trim_end_matches('/') {
    return Some("");
  }
  path.strip_prefix(base)
}

/// 目录列表用的 URL 前缀（含 base href，以 `/` 结尾，未编码）。
pub fn url_prefix(base_href: Option<&str>, rel: &str) -> String {
  let base = base_href.map(|b| b.trim_end_matches('/')).unwrap_or("");
  let mut prefix = format!("{base}/{rel}");
  if !prefix.ends_with('/') {
    prefix.push('/');
  }
  prefix
}

/// 处理一个请求：路由、日志、HEAD 语义都在这里收口。
pub async fn handle_request(req: Request<Incoming>, config: AppConfig) -> Response<ResBody> {
  let raw_path = req.uri().path().to_string();
  let is_head = req.method() == Method::HEAD;

  let mut res = route(&req, &config, &raw_path).await;
  if is_head {
    *res.body_mut() = crate::helper::empty_body();
  }
  if config.should_log_access {
    log_access(res.status().as_u16(), &raw_path);
  }
  res
}

async fn route(req: &Request<Incoming>, config: &Config, raw_path: &str) -> Response<ResBody> {
  if config.delay > 0 {
    // `--delay` is documented in milliseconds, keep the unit consistent.
    tokio::time::sleep(Duration::from_millis(config.delay)).await;
  }

  let mut url_path: &str = raw_path;

  if let Some(base) = config.base_href.as_ref().filter(|p| !is_empty_root_url(p)) {
    if is_empty_root_url(raw_path) {
      let mut res = response(StatusCode::FOUND);
      res.headers_mut().insert(
        LOCATION,
        HeaderValue::from_str(base).unwrap_or_else(|_| HeaderValue::from_static("/")),
      );
      return res;
    }
    match strip_base_href(raw_path, base) {
      Some(rest) => url_path = rest,
      None => return response(StatusCode::FORBIDDEN),
    }
  }

  let rel = match sanitize_path(url_path) {
    Ok(rel) => rel,
    Err(PathError::BadRequest) => return response(StatusCode::BAD_REQUEST),
    Err(PathError::Forbidden) => return response(StatusCode::FORBIDDEN),
  };

  let file_path = config.root_dir.join(&rel);

  // 只有“看起来像静态文件”的 URL（最后一段带扩展名、且不以 / 结尾）才直接 404，
  // 其余交给 html5 的 index 回退（回退到根目录的 index.html）。
  let is_file_like = !url_path.ends_with('/') && Path::new(&rel).extension().is_some();

  match tokio::fs::metadata(&file_path).await {
    Ok(stat) => {
      // 阻断符号链接逃逸：root/link -> /etc/passwd
      if !is_inside_root(&config.root_canonical, &file_path).await {
        return response(StatusCode::FORBIDDEN);
      }

      if stat.is_dir() {
        // 目录默认尝试 <dir>/index.html；`--list-dir` 时跳过硬编码的 index，直接列目录。
        if !config.list_dir {
          let index_path = file_path.join(INDEX_FILE);
          if let Ok(index_stat) = tokio::fs::metadata(&index_path).await {
            if index_stat.is_file() && is_inside_root(&config.root_canonical, &index_path).await {
              return send_file_304(&index_stat, req, &index_path, config.gzip).await;
            }
          }
        }
        let prefix = url_prefix(config.base_href.as_deref(), &rel);
        return match send_dir(&file_path, &prefix).await {
          Ok(res) => res,
          Err(err) => {
            crate::error!("failed to list {}: {}", file_path.display(), err);
            response(StatusCode::INTERNAL_SERVER_ERROR)
          }
        };
      }

      if stat.is_file() {
        return send_file_304(&stat, req, &file_path, config.gzip).await;
      }

      response(StatusCode::NOT_FOUND)
    }
    Err(_) => {
      if !config.html5 || is_file_like {
        return response(StatusCode::NOT_FOUND);
      }
      // html5 回退：返回根目录的 index.html
      let index_path: &PathBuf = &config.index_href;
      match tokio::fs::metadata(index_path).await {
        Ok(stat) => send_file_304(&stat, req, index_path, config.gzip).await,
        Err(_) => response(StatusCode::NOT_FOUND),
      }
    }
  }
}

async fn send_file_304(
  stat: &std::fs::Metadata,
  req: &Request<Incoming>,
  file_path: &Path,
  gzip: bool,
) -> Response<ResBody> {
  let mtime = stat
    .modified()
    .ok()
    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|elapsed| elapsed.as_secs())
    .unwrap_or(0)
    .to_string();
  let etag = etag_for(&mtime);

  if !should_send_file(req.headers(), &etag) {
    let mut res = response(StatusCode::NOT_MODIFIED);
    res.headers_mut().insert(
      ETAG,
      HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("\"\"")),
    );
    return res;
  }

  let gzip = gzip && accepts_gzip(req.headers());
  match send_file(file_path, stat.len(), &etag, gzip).await {
    Ok(res) => res,
    Err(err) => {
      crate::error!("failed to open {}: {}", file_path.display(), err);
      response(StatusCode::INTERNAL_SERVER_ERROR)
    }
  }
}
