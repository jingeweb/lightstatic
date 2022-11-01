extern crate core;

mod args;
mod config;
mod helper;
mod logger;
mod pid;
mod server;
mod server_core;
mod store;
mod util;

use crate::logger::initialize_log;
use crate::pid::handle_arg_signal;
use async_std::path::PathBuf;

#[async_std::main]
async fn main() -> tide::Result<()> {
  let args = args::get_args();
  let cwd = PathBuf::from(std::env::current_dir().unwrap());
  initialize_log(&args, &cwd);
  if !args
    .signal
    .as_ref()
    .map(|s| handle_arg_signal(s))
    .unwrap_or(false)
  {
    server::bootstrap(args, cwd).await
  } else {
    Ok(())
  }
}
