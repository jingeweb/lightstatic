use crate::config::{AppConfig, Config};
use crate::helper::{send_dir, send_file, should_send_file};
use crate::logger::log_access;
use crate::store::send_cache_file;
use crate::util::is_empty_root_url;
use async_std::path::Path;
use std::fs::Metadata;
use std::ops::Deref;
use std::{future::Future, pin::Pin};
use tide::{Next, Request, Response, StatusCode};

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
  if should_send_file(req, &mtime)? {
    send(
      send_file(file_path, stat, &mtime, gzip).await,
      should_log_access,
      path,
    )
  } else {
    send(Ok(Response::new(304)), should_log_access, path)
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
      ref base_href,
      html5,
      ref index_href,
      ref cache_store,
      gzip,
      ..
    } = req.state().deref();
    if delay > 0 {
      async_std::task::sleep(std::time::Duration::from_secs(delay)).await;
    }

    let path = req.url().path();
    let url;

    if let Some(base_href) = base_href.as_ref().filter(|p| !is_empty_root_url(*p)) {
      // println!("with base href");
      if is_empty_root_url(path) {
        if should_log_access {
          log_access(200, path)
        };
        return Ok(Response::builder(302).header("location", base_href).build());
      } else if !Path::new(path).starts_with(base_href) {
        if should_log_access {
          log_access(403, path)
        };
        return Ok(Response::new(StatusCode::Forbidden));
      } else {
        url = if path.len() > base_href.len() {
          &path[base_href.len()..]
        } else {
          &path[0..0]
        };
      }
    } else {
      url = &path[1..];
    }

    let file_path = root_dir.join(url);

    if let Some(cache_store) = cache_store {
      return send(
        send_cache_file(&req, cache_store, &file_path, html5).await,
        should_log_access,
        path,
      );
    }

    // println!("{} {:?} {:?}", url, root_dir, root_dir.join(url));

    match file_path.metadata().await {
      Ok(stat) => {
        if stat.is_dir() {
          send(send_dir(&file_path, path).await, should_log_access, path)
        } else if stat.is_file() {
          send_file_304(&stat, &req, &file_path, should_log_access, path, gzip).await
        } else {
          send(Ok(Response::new(404)), should_log_access, path)
        }
      }
      Err(_) => {
        if !html5 || url.chars().rev().any(|c| c == '.') {
          send(Ok(Response::new(404)), should_log_access, path)
        } else {
          let index_href = Path::new(index_href.as_path());
          match index_href.metadata().await {
            Ok(ref stat) => {
              send_file_304(stat, &req, index_href, should_log_access, path, gzip).await
            }
            Err(err) => send(
              Err(http_types::Error::new(StatusCode::InternalServerError, err)),
              should_log_access,
              path,
            ),
          }
        }
      }
    }
  })
}
