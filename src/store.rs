use crate::config::AppConfig;
use crate::error;
use crate::helper::{get_mime_from_ext, should_send_file};
use ahash::AHashMap;
use async_std::fs::File;
use async_std::io::{ReadExt, Result};
use async_std::path::{Path, PathBuf};
use async_std::stream::StreamExt;
use async_std::sync::RwLock;
use futures::future::{BoxFuture, FutureExt};
use futures::{AsyncBufRead, AsyncRead};
use http_types::Body;
use regex::Regex;
use std::io::{ErrorKind, Write};
use std::ops::Deref;
use std::sync::Arc;
use std::task::Poll;
// use std::time::SystemTime;
// use async_compression::futures::write::GzipEncoder;
// use async_compression::Level;
use flate2::write::GzEncoder;
use flate2::Compression;
use tide::{Request, Response};

pub struct FileCache {
  buffer: Arc<Vec<u8>>,
  gzipped: bool,
  file_ext: Option<String>,
  mtime: String,
  cache_forever: bool,
}

impl FileCache {
  fn to_reader(&self) -> FileCacheReader {
    FileCacheReader {
      buffer: self.buffer.clone(),
      bytes_read: 0,
    }
  }
}

pub struct FileCacheReader {
  buffer: Arc<Vec<u8>>,
  bytes_read: usize,
}

impl AsyncRead for FileCacheReader {
  fn poll_read(
    mut self: std::pin::Pin<&mut Self>,
    _: &mut std::task::Context<'_>,
    buf: &mut [u8],
  ) -> Poll<Result<usize>> {
    // println!("poll_read {}", buf.len());
    match std::io::Read::read(
      &mut &self.buffer.as_ref()[(self.bytes_read as usize)..],
      buf,
    ) {
      Ok(size) => {
        self.bytes_read += size;
        // println!("poll_read ok {}", size);
        Poll::Ready(Ok(size))
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}
impl AsyncBufRead for FileCacheReader {
  fn poll_fill_buf(
    self: std::pin::Pin<&mut Self>,
    _: &mut std::task::Context<'_>,
  ) -> Poll<Result<&[u8]>> {
    println!("poll_fill_buf");
    let rd = self.get_mut();
    Poll::Ready(Ok(&rd.buffer.deref()[rd.bytes_read..]))
  }
  fn consume(mut self: std::pin::Pin<&mut Self>, amt: usize) {
    self.bytes_read += amt;
    println!(
      "consume poll fill buf {} {} {}",
      amt,
      self.bytes_read,
      self.buffer.len()
    );
  }
}

type HMap = AHashMap<PathBuf, FileCache>;
pub struct FileCacheStore {
  store: HMap,
  index_file: FileCache,
  index_href: PathBuf,
  root_dir: PathBuf,
  regex_immutable: Option<Regex>,
}

impl FileCacheStore {
  pub fn len(&self) -> usize {
    self.store.len()
  }
}

pub async fn refresh_cache_store(store: &RwLock<FileCacheStore>) -> Option<usize> {
  let rd_store = store.read().await;
  if let Some(index_file) = read_file(
    Path::new(&rd_store.index_href),
    rd_store.regex_immutable.as_ref(),
  )
  .await
  {
    drop(rd_store); // 释放 read lock，否则接下来的 write 会死锁
    let mut wt_store = store.write().await;
    wt_store.index_file = index_file;
    drop(wt_store); // 释放 write lock，否则后续的 read 会死锁
  } else {
    // return 会自动释放 read lock
    return None;
  }

  let mut new_store = HMap::new();
  let rd_store = store.read().await;
  if !(loop_read_dir(
    &rd_store.root_dir,
    &mut new_store,
    rd_store.regex_immutable.as_ref(),
  )
  .await)
  {
    // return 会自动释放 read lock
    return None;
  } else {
    // 释放 read lock，否则接下来的 write 会死锁
    drop(rd_store);
  }

  let mut wt_store = store.write().await;
  wt_store.store.clear();
  for (fp, cf) in new_store.into_iter() {
    wt_store.store.insert(fp, cf); // 旧的 CacheFile 会被返回，然后自动 drop 清理
  }
  // return 会自动释放 write lock
  Some(wt_store.len())
}

async fn read_file(file_path: &Path, cache_forever_regexp: Option<&Regex>) -> Option<FileCache> {
  match async move {
    let stat = file_path.metadata().await?;
    let mut file = File::open(file_path).await?;
    let mut buf: Vec<u8> = Vec::with_capacity(stat.len() as usize);
    // let s = SystemTime::now();
    file.read_to_end(&mut buf).await?;
    // println!("end read file {}", SystemTime::now().duration_since(s).unwrap().as_secs());
    // let s = SystemTime::now();
    // let mut gzip = GzipEncoder::with_quality(Vec::new(), Level::Best);
    // gzip.write_all(&*buf).await?;
    // gzip.close().await?;
    // let c_buf = gzip.into_inner();
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(&*buf)?;
    let c_buf = gz.finish()?;
    // println!("end gzip file {}", SystemTime::now().duration_since(s).unwrap().as_secs());

    let (gzipped, buffer) = if c_buf.len() < buf.len() {
      (true, c_buf)
    } else {
      (false, buf)
    };
    let mtime = stat
      .modified()
      .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?
      .duration_since(std::time::UNIX_EPOCH)
      .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?
      .as_secs()
      .to_string();

    // println!("read_file {:?} {} {} {}", file_path, gzipped, buffer.len(), mtime);
    Result::<FileCache>::Ok(FileCache {
      buffer: Arc::new(buffer),
      gzipped,
      file_ext: file_path
        .extension()
        .and_then(|p| p.to_str())
        .map(String::from),
      mtime,
      cache_forever: file_path
        .to_str()
        .and_then(|t| cache_forever_regexp.map(|r| r.is_match(t)))
        .unwrap_or(false),
    })
  }
  .await
  {
    Ok(fc) => Some(fc),
    Err(err) => {
      error!("failed read file {} due to {}", file_path.display(), err);
      None
    }
  }
}

fn loop_read_dir<'a: 'f, 'f>(
  dir: &'a Path,
  store: &'a mut HMap,
  regex_immutable: Option<&'a Regex>,
) -> BoxFuture<'f, bool> {
  async move {
    match async move {
      let mut rd = dir.read_dir().await?;
      while let Some(rd) = rd.next().await {
        let rd = rd?;
        let file_path = rd.path();
        let stat = file_path.metadata().await?;
        if stat.is_dir() {
          loop_read_dir(&file_path, store, regex_immutable).await;
        } else if stat.is_file() {
          if let Some(cache_file) = read_file(&file_path, regex_immutable).await {
            store.insert(file_path, cache_file);
          }
        }
      }
      Result::<()>::Ok(())
    }
    .await
    {
      Ok(_) => true,
      Err(err) => {
        error!("failed load dir {} due to {}", dir.display(), err);
        false
      }
    }
  }
  .boxed()
}

pub async fn init_cache_store(
  index_href: PathBuf,
  root_dir: PathBuf,
  regex_immutable: Option<Regex>,
) -> FileCacheStore {
  let index_file = read_file(Path::new(&index_href), regex_immutable.as_ref())
    .await
    .unwrap();

  let mut store = HMap::new();
  if !(loop_read_dir(Path::new(&root_dir), &mut store, regex_immutable.as_ref()).await) {
    panic!("failed to read static dir");
  }

  FileCacheStore {
    store,
    index_file,
    index_href,
    root_dir,
    regex_immutable,
  }
}

pub async fn send_cache_file(
  req: &Request<AppConfig>,
  store: &RwLock<FileCacheStore>,
  file_path: &Path,
  html5: bool,
) -> tide::Result<Response> {
  let rd_store = store.read().await;
  // println!("{:?}", file_path);
  let cache_file = match &rd_store.store.get(file_path) {
    Some(file) => file,
    None => {
      if !html5 {
        return Ok(Response::new(404));
      }
      &rd_store.index_file
    }
  };
  // println!("{}", cache_file.buffer.len());
  Ok(if should_send_file(req, &cache_file.mtime)? {
    let mut res = Response::builder(200).header("etag", &cache_file.mtime);
    if cache_file.cache_forever {
      res = res.header("cache-control", "max-age=31536000, immutable");
    }
    if cache_file.gzipped {
      res = res.header("content-encoding", "gzip");
    }

    let mut body = Body::from_reader(cache_file.to_reader(), Some(cache_file.buffer.len()));
    body.set_mime(get_mime_from_ext(
      cache_file.file_ext.as_ref().map(|s| &s[..]),
    ));
    res.body(body).build()
  } else {
    Response::new(304)
  })
}

#[async_std::test]
async fn test_cache() -> Result<()> {
  panic!("")
}
