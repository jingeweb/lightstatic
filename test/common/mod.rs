//! 集成测试共用的工具：临时目录、端口分配、起服务、极简 HTTP 客户端。
#![allow(dead_code)]

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// 每个测试独立的临时目录（用完不清理，系统会回收 /tmp）。
pub fn temp_dir(tag: &str) -> PathBuf {
  static COUNTER: AtomicU32 = AtomicU32::new(0);
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir().join(format!(
    "lightstatic-test-{tag}-{}-{nanos}-{seq}",
    std::process::id()
  ));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

pub fn write_file(root: &Path, rel: &str, content: &[u8]) -> PathBuf {
  let path = root.join(rel);
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent).unwrap();
  }
  std::fs::write(&path, content).unwrap();
  path
}

/// 并行测试里保证本进程内不会把同一个端口分配两次。
static ALLOCATED_PORTS: LazyLock<Mutex<HashSet<u16>>> =
  LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn free_port() -> u16 {
  let mut allocated = ALLOCATED_PORTS.lock().unwrap();
  loop {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    if allocated.insert(port) {
      return port;
    }
  }
}

pub struct TestServer {
  child: Child,
  pub port: u16,
}

impl TestServer {
  /// 起一个实例，并从它的启动日志里读出**真实**监听端口。
  ///
  /// 端口被抢占时 `--port` 会让程序自动选下一个可用端口，所以不能假定就是我们请求的那个。
  /// 启动失败（极小的端口竞争窗口）时换端口重试。
  pub fn start(root: &Path, extra: &[&str]) -> TestServer {
    let mut last_error = String::new();
    for _ in 0..5 {
      match TestServer::try_start(root, extra) {
        Ok(server) => return server,
        Err(err) => {
          last_error = err;
          std::thread::sleep(Duration::from_millis(100));
        }
      }
    }
    panic!("failed to start lightstatic: {last_error}");
  }

  fn try_start(root: &Path, extra: &[&str]) -> Result<TestServer, String> {
    let requested = free_port();

    let mut child = Command::new(env!("CARGO_BIN_EXE_lightstatic"))
      .arg(root)
      .arg("--host")
      .arg("127.0.0.1")
      .arg("--port")
      .arg(requested.to_string())
      .args(extra)
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .spawn()
      .expect("failed to spawn lightstatic");

    let stdout = child.stdout.take().expect("child stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
      // 持续排空 stdout（访问日志也走这里），只在第一行 URL 上回报端口。
      let mut reported = false;
      for line in BufReader::new(stdout).lines().map_while(|line| line.ok()) {
        if reported {
          continue;
        }
        if let Some(rest) = line.split("http://127.0.0.1:").nth(1) {
          // 行尾可能还跟着 "  with base href /app/"
          let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
          if let Ok(port) = digits.parse::<u16>() {
            reported = true;
            let _ = sender.send(port);
          }
        }
      }
    });

    let port = match receiver.recv_timeout(Duration::from_secs(10)) {
      Ok(port) => port,
      Err(_) => {
        let _ = child.kill();
        let _ = child.wait();
        return Err("did not report a listening port".to_string());
      }
    };

    let mut server = TestServer { child, port };
    if !server.wait_ready() {
      let _ = server.child.kill();
      let _ = server.child.wait();
      return Err(format!("port {port} was not reachable"));
    }
    Ok(server)
  }

  fn wait_ready(&self) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
      if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
        return true;
      }
      std::thread::sleep(Duration::from_millis(30));
    }
    false
  }

  pub fn get(&self, path: &str) -> HttpResponse {
    self.request(path, &[])
  }

  pub fn request(&self, path: &str, headers: &[(&str, &str)]) -> HttpResponse {
    http_request(self.port, path, headers)
  }
}

impl Drop for TestServer {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

#[derive(Debug)]
pub struct HttpResponse {
  pub status: u16,
  pub headers: Vec<(String, String)>,
  pub body: Vec<u8>,
}

impl HttpResponse {
  pub fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(key, _)| key.eq_ignore_ascii_case(name))
      .map(|(_, value)| value.as_str())
  }

  pub fn text(&self) -> String {
    String::from_utf8_lossy(&self.body).to_string()
  }
}

/// 手写 HTTP/1.1 请求：这样才能原样发送 `//etc/passwd` 这类路径
/// （curl 默认会做路径归一化）。
///
/// 并行跑测试时端口分配有极小概率撞车，这里对连接类错误做几次重试。
pub fn http_request(port: u16, path: &str, headers: &[(&str, &str)]) -> HttpResponse {
  http_request_with_method(port, "GET", path, headers)
}

pub fn http_request_with_method(
  port: u16,
  method: &str,
  path: &str,
  headers: &[(&str, &str)],
) -> HttpResponse {
  let mut last_error = None;
  for attempt in 0..5 {
    match try_request(port, method, path, headers) {
      Ok(response) => return response,
      Err(err) => {
        last_error = Some(err);
        std::thread::sleep(Duration::from_millis(50 * (attempt + 1)));
      }
    }
  }
  panic!("request {path} failed: {last_error:?}");
}

fn try_request(
  port: u16,
  method: &str,
  path: &str,
  headers: &[(&str, &str)],
) -> std::io::Result<HttpResponse> {
  let mut stream = TcpStream::connect(("127.0.0.1", port))?;
  stream.set_read_timeout(Some(Duration::from_secs(10)))?;
  stream.set_write_timeout(Some(Duration::from_secs(10)))?;

  let mut request =
    format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
  for (name, value) in headers {
    request.push_str(&format!("{name}: {value}\r\n"));
  }
  request.push_str("\r\n");
  stream.write_all(request.as_bytes())?;

  let mut raw = Vec::new();
  stream.read_to_end(&mut raw)?;
  Ok(parse_response(&raw))
}

fn parse_response(raw: &[u8]) -> HttpResponse {
  let split = raw
    .windows(4)
    .position(|window| window == b"\r\n\r\n")
    .expect("response without header terminator");

  let head = String::from_utf8_lossy(&raw[..split]).to_string();
  let mut lines = head.split("\r\n");
  let status = lines
    .next()
    .and_then(|line| line.split_whitespace().nth(1))
    .and_then(|code| code.parse().ok())
    .expect("bad status line");

  let headers: Vec<(String, String)> = lines
    .filter_map(|line| {
      let (name, value) = line.split_once(':')?;
      Some((name.trim().to_string(), value.trim().to_string()))
    })
    .collect();

  let mut body = raw[split + 4..].to_vec();
  let chunked = headers.iter().any(|(name, value)| {
    name.eq_ignore_ascii_case("transfer-encoding") && value.to_ascii_lowercase().contains("chunked")
  });
  if chunked {
    body = dechunk(&body);
  } else if let Some(len) = headers
    .iter()
    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    .and_then(|(_, value)| value.parse::<usize>().ok())
  {
    body.truncate(len);
  }

  HttpResponse {
    status,
    headers,
    body,
  }
}

fn dechunk(data: &[u8]) -> Vec<u8> {
  let mut out = Vec::new();
  let mut rest = data;
  while let Some(pos) = rest.windows(2).position(|window| window == b"\r\n") {
    let size = String::from_utf8_lossy(&rest[..pos]);
    let size = usize::from_str_radix(size.split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
    rest = &rest[pos + 2..];
    if size == 0 {
      break;
    }
    let end = size.min(rest.len());
    out.extend_from_slice(&rest[..end]);
    rest = &rest[(end + 2).min(rest.len())..];
  }
  out
}

pub fn gunzip(data: &[u8]) -> Vec<u8> {
  let mut decoder = flate2::read::GzDecoder::new(data);
  let mut out = Vec::new();
  decoder.read_to_end(&mut out).expect("invalid gzip payload");
  out
}
