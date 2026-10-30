# lightstatic

> lightweight static file server

> ⚠️ **仅供本地开发、演示和内网调试使用，请勿部署到生产环境。**
> 它没有做安全加固、访问控制、限流与性能压测；生产环境的静态站点请使用 nginx / caddy 等成熟服务器。

## 安装

````bash
# npm（发布包里自带预编译二进制，无需 Rust 工具链）
npm install -g lightstatic

# 或者从源码编译安装
cargo install lightstatic
````

预编译二进制覆盖 macOS（x64 / arm64）、Linux（x64 / arm64）和 Windows（x64）。

## 使用

````
Usage: lightstatic [OPTIONS] [PATH]

Arguments:
  [PATH]  directory path to serve

Options:
  -H, --host <IP>              ip address to bind (default: "0.0.0.0") [default: 0.0.0.0]
  -p, --port <PORT>            port to listen (default: "8080"). if the specified port is not available, find a free port instead [default: 8080]
  -g, --gzip                   gzip encode response content (default: false)
  -o, --open                   open browser window after starting the server (default: false)
  -5, --html5                  use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
      --list-dir               list directories even when they contain index.html (default: false)
  -d, --delay <DELAY>          delay in milliseconds for response (default: "0") [default: 0]
  -l, --log-dir <DIRECTORY>    write logs to directory, if specified
  -b, --base-href <BASE_HREF>  server base href, useful when under nginx sub path
  -A, --no-access              do not print access log
  -C, --no-color               disable color log
  -h, --help                   Print help
  -V, --version                Print version
````

## 示例

````bash
lightstatic ./dist -5 -o            # SPA 本地开发：history 路由回退 + 自动开浏览器
lightstatic ./dist -b /app/ -l ./log # 挂在 nginx 子路径下，并把访问日志落盘
````

## 开发

见 [DEVELOPMENT.md](DEVELOPMENT.md)（构建、测试、发布流程）。
