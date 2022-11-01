use std::thread::sleep;
use std::time::Duration;
use std::{
  env::temp_dir,
  fs::{self, read_to_string, File},
  io::{self, Write},
  process,
};

use libc::{kill, pid_t, SIGHUP, SIGTERM};

use crate::{error, info};

const PID_FILE: &str = "lightstatic.pid";

pub fn handle_arg_signal(action: &str) -> bool {
  // if let Some(action) = &args.signal {
  let sig = match action.to_lowercase().as_str() {
    "stop" => SIGTERM,
    "refresh" => SIGHUP,
    _ => {
      error!("error: option --signal only accept \"stop\" or \"refresh\" action");
      process::exit(-1);
    }
  };
  // println!("{:?}", temp_dir().join(PID_FILE));
  if let Ok(cnt) = read_to_string(temp_dir().join(PID_FILE)) {
    let pids: Vec<_> = cnt
      .split('\n')
      .filter(|pid| {
        if let Ok(pid) = pid.parse::<pid_t>() {
          let r = unsafe { kill(pid, sig) };
          sleep(Duration::from_secs(1)); // 目标进程退出时也会从 pid file 中删除自身 pid，所以等待 1 秒让所有目标进程处理完后再操作。
          info!("Send signal {} to pid {}, result {}", sig, pid, r);
          r == 0 && sig != SIGTERM
        } else {
          false
        }
      })
      .collect();

    handle_pids(&pids);
    true
  } else {
    error!(
      "error: {} not found, no lightstatic process is running.",
      PID_FILE
    );
    process::exit(-1);
  }
}

pub fn write_pid() -> io::Result<()> {
  let mut f = File::options()
    .append(true)
    .create(true)
    .open(temp_dir().join(PID_FILE))?;
  let mut pid = process::id().to_string();
  if f.metadata()?.len() > 0 {
    pid.insert(0, '\n');
  }
  f.write_all(pid.as_bytes())
}

fn handle_pids(pids: &Vec<&str>) {
  let pid_file = temp_dir().join(PID_FILE);
  if pids.is_empty() {
    if pid_file.exists() {
      // println!("remove pid file");
      fs::remove_file(temp_dir().join(&pid_file))
    } else {
      Ok(())
    }
  } else {
    File::options()
      .write(true)
      .truncate(true)
      .open(&pid_file)
      .and_then(|mut f| {
        // let x = pids.join("\n");
        // println!("update pid file: {:?} {}", pids, x);
        f.write_all(pids.join("\n").as_bytes())
      })
  }
  .unwrap_or_else(|err| {
    sleep(Duration::from_secs(2));
    error!(
      "failed {} pid file due to {}",
      if !pids.is_empty() { "update" } else { "remove" },
      err
    )
  })
}

pub fn remove_pid() {
  fn remove() -> io::Result<()> {
    let cnt = read_to_string(temp_dir().join(PID_FILE))?;
    let pid = process::id().to_string();
    let pids: Vec<_> = cnt
      .split('\n')
      .filter(|l| !l.trim().is_empty())
      .filter(|l| !l.eq(&pid))
      .collect();
    // println!("will remove {} {:?}", pid, pids);
    handle_pids(&pids);
    Ok(())
  }
  if let Err(err) = remove() {
    error!("failed remove pid: {}", err);
  }
}
