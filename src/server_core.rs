use crate::config::{AppConfig, Config};
use crate::helper::{accepts_gzip, etag_for, send_dir, send_file, should_send_file};
use crate::logger::log_access;
use crate::util::{is_empty_root_url, is_inside_root};
use async_std::path::Path;
use percent_encoding::percent_decode_str;
use std::fs::Metadata;
use std::ops::Deref;
use std::{future::Future, pin::Pin};
use tide::{Next, Request, Response, StatusCode};

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

fn path_error_response(err: PathError) -> Response {
  match err {
    PathError::BadRequest => Response::new(StatusCode::BadRequest),
    PathError::Forbidden => Response::new(StatusCode::Forbidden),
  }
}

fn send(res: tide::Result, should_log_access: bool, path: &str) -> tide::Result {
  if !should_log_access {
    return res;
  }
  let code = match &res {
    Ok(res) => res.status() as u16,
    Err(err) => {
      crate::error!("{}", err);
      500
    }
  };
  log_access(code, path);
  res
}

async fn send_file_304(
  stat: &Metadata,
  req: &Request<AppConfig>,
  file_path: &Path,
  should_log_access: bool,
  path: &str,
  gzip: bool,
) -> tide::Result {
  let mtime = stat
    .modified()?
    .duration_since(std::time::UNIX_EPOCH)?
    .as_secs()
    .to_string();
  let etag = etag_for(&mtime);
  if should_send_file(req.header("if-none-match"), &etag) {
    let gzip = gzip && accepts_gzip(req.header("accept-encoding"));
    send(
      send_file(file_path, stat, &etag, gzip).await,
      should_log_access,
      path,
    )
  } else {
    send(
      Ok(Response::builder(304).header("etag", etag).build()),
      should_log_access,
      path,
    )
  }
}

pub fn handle_request<'a>(
  req: Request<AppConfig>,
  _: Next<'a, AppConfig>,
) -> Pin<Box<dyn Future<Output = tide::Result> + Send + 'a>> {
  Box::pin(async move {
    let &Config {
      delay,
      should_log_access,
      ref root_dir,
      ref root_canonical,
      ref base_href,
      html5,
      ref index_href,
      gzip,
    } = req.state().deref();

    if delay > 0 {
      // `--delay` is documented in milliseconds, keep the unit consistent.
      async_std::task::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let raw_path = req.url().path().to_string();
    let mut url_path: &str = &raw_path;

    if let Some(base) = base_href.as_ref().filter(|p| !is_empty_root_url(p)) {
      if is_empty_root_url(&raw_path) {
        if should_log_access {
          log_access(302, &raw_path);
        }
        return Ok(
          Response::builder(302)
            .header("location", base.as_str())
            .build(),
        );
      }
      match strip_base_href(&raw_path, base) {
        Some(rest) => url_path = rest,
        None => {
          if should_log_access {
            log_access(403, &raw_path);
          }
          return Ok(Response::new(StatusCode::Forbidden));
        }
      }
    }

    let rel = match sanitize_path(url_path) {
      Ok(rel) => rel,
      Err(err) => {
        let res = path_error_response(err);
        if should_log_access {
          log_access(res.status() as u16, &raw_path);
        }
        return Ok(res);
      }
    };

    let file_path = root_dir.join(&rel);

    // 只有“看起来像静态文件”的 URL（最后一段带扩展名、且不以 / 结尾）才直接 404，
    // 其余交给 html5 的 index 回退。
    let is_file_like = !url_path.ends_with('/') && Path::new(&rel).extension().is_some();

    match file_path.metadata().await {
      Ok(stat) => {
        // 阻断符号链接逃逸：root/link -> /etc/passwd
        if !is_inside_root(root_canonical, &file_path).await {
          if should_log_access {
            log_access(403, &raw_path);
          }
          return Ok(Response::new(StatusCode::Forbidden));
        }

        if stat.is_dir() {
          let prefix = url_prefix(base_href.as_deref(), &rel);
          send(
            send_dir(&file_path, &prefix).await,
            should_log_access,
            &raw_path,
          )
        } else if stat.is_file() {
          send_file_304(&stat, &req, &file_path, should_log_access, &raw_path, gzip).await
        } else {
          send(
            Ok(Response::new(StatusCode::NotFound)),
            should_log_access,
            &raw_path,
          )
        }
      }
      Err(_) => {
        if !html5 || is_file_like {
          send(
            Ok(Response::new(StatusCode::NotFound)),
            should_log_access,
            &raw_path,
          )
        } else {
          let index_path = Path::new(index_href.as_path());
          match index_path.metadata().await {
            Ok(stat) => {
              send_file_304(&stat, &req, index_path, should_log_access, &raw_path, gzip).await
            }
            Err(err) => send(
              Err(http_types::Error::new(StatusCode::InternalServerError, err)),
              should_log_access,
              &raw_path,
            ),
          }
        }
      }
    }
  })
}
