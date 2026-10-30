use lightstatic::args;
use lightstatic::logger::initialize_log;
use lightstatic::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = args::get_args();
  let cwd = std::env::current_dir()?;
  initialize_log(&args, &cwd);
  server::bootstrap(args, cwd).await
}
