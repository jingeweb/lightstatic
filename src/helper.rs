use async_compression::futures::bufread::GzipEncoder;
use async_std::{
  fs,
  io::{self, BufReader},
  path::Path,
  stream::StreamExt,
};
use http_types::{mime, Mime};
use tide::{Body, Request, Response};

use crate::config::AppConfig;

pub async fn send_dir(dir: &Path, path: &str) -> tide::Result<Response> {
  let mut list = dir.read_dir().await?;
  let mut cnt = String::new();
  while let Some(item) = list.next().await {
    let item = item?.path();
    let file_name = item
      .file_name()
      .and_then(|s| s.to_str())
      .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "bad file"))?;
    cnt.push_str(&format!(
      "  <li><a href=\"{1}{2}{0}\">{0}</a></li>\n",
      file_name,
      path,
      if path.ends_with('/') { "" } else { "/" } // base_href.unwrap_or(""),
                                                 // base_url,
                                                 // if base_url.ends_with('/') { "" } else { "/" }
    ));
  }
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
    path, cnt
  );
  Ok(
    Response::builder(200)
      .content_type("text/html")
      .body(cnt)
      .build(),
  )
}

pub fn get_mime(file: &Path) -> Mime {
  get_mime_from_ext(file.extension().and_then(|p| p.to_str()))
}

pub fn get_mime_from_ext(ext: Option<&str>) -> Mime {
  ext
    .and_then(Mime::from_extension)
    .unwrap_or(mime::BYTE_STREAM)
}

pub async fn send_file(
  filepath: &Path,
  stat: &std::fs::Metadata,
  mtime: &str,
  gzip: bool,
) -> tide::Result<Response> {
  let file = fs::File::open(filepath).await?;
  let mut body;
  let mut res = Response::builder(200).header("etag", mtime);
  if gzip {
    res = res.header("content-encoding", "gzip");
    body = Body::from_reader(BufReader::new(GzipEncoder::new(BufReader::new(file))), None);
  } else {
    body = Body::from_reader(BufReader::new(file), Some(stat.len() as usize));
  }
  body.set_mime(get_mime(filepath));
  Ok(res.body(body).build())
}

pub fn should_send_file(req: &Request<AppConfig>, mtime: &str) -> tide::Result<bool> {
  let if_none_match = req.header("if-none-match").and_then(|ims| ims.get(0));
  Ok(if_none_match.filter(|ims| ims.as_str().eq(mtime)).is_none())
}
