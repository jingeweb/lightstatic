# lightstatic

> lightweight static file server for development purpose

## Install

````bash
npm install -g lightstatic
````

## Usage

````
Usage: lightstatic [path] [options]

Options:
  -V, --version        output the version number
  -b, --bind <string>  ip address to bind (default: "0.0.0.0")
  -p, --port <number>  port to listen. if the specified port is not avaiable, sfserver will find a free port instead (default: 8080)
  -g, --gzip           gzip encode response content (default: false)
  -o, --open           open browser window after starting the server (default: false)
  -5, --html5          use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
  --index <file>       index file to redirect under html5 mode (default: "index.html")
  --base-href <path>   server base href, useful when under nginx subpath
  --no-access          do not print access log
  --no-log             do not print any log expect errors
  --no-color           disable color log
  -h, --help           display help for command

Examples:
  lightstatic
  lightstatic ./dist -g -5
````
