use async_std::path::PathBuf;

use lightstatic::args;
use lightstatic::logger::initialize_log;
use lightstatic::server;

#[async_std::main]
async fn main() -> tide::Result<()> {
  let args = args::get_args();
  let cwd = PathBuf::from(std::env::current_dir().unwrap());
  initialize_log(&args, &cwd);
  server::bootstrap(args, cwd).await
}
