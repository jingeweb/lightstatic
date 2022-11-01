# lightstatic

> lightweight static file server

## Install

````bash
cargo install lightstatic
````

## Usage

````
USAGE:
    lightstatic [OPTIONS] [PATH]

ARGS:
    <PATH>    directory path to serve

OPTIONS:
    -5, --html5                       use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
    -A, --no-access                   do not print access log
    -b, --base-href <BASE_HREF>       server base href, useful when under nginx sub path
    -c, --cache-in-memory             store(cache) static files into memory (default: false)
    -C, --no-color                    disable color log
    -d, --delay <DELAY>               delay in milliseconds for response (default: "0") [default: 0]
    -g, --gzip                        gzip encode response content (default: false)
    -h, --help                        print help information
    -H, --host <IP>                   ip address to bind (default: "0.0.0.0") [default: 0.0.0.0]
    -i, --index <FILE>                index file to redirect under html5 mode (default: "index.html") [default: index.html]
    -l, --log-dir <DIRECTORY>         write logs to directory, if specified
    -o, --open                        open browser window after starting the server (default: false)
    -p, --port <PORT>                 port to listen (default: "8080"). if the specified port is not available, find a free port instead [default: 8080]
    -r, --regex-immutable <REGEXP>    cache files which match regexp forever, if specified
    -s, --signal <ACTION>             send signal to running process, action can be "stop" or "refresh"
    -V, --version                     Print version information
````

## Examples

````bash
lightstatic ./dist -5 -o # for local development server
lightstatic -5 -c -l ./log --immutable '\w+\.[0-9a-z]{16}\.(js|css|png|svg|jpg)$' # for online static spa server
````
