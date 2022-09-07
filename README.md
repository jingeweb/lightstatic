# lightstatic

> lightweight static file server

## Install

````bash
cargo install lightstatic
````

## Usage

````
Usage: lightstatic [path] [options]

Options:
  -V, --version                        output the version number
  -b, --bind <string>                  ip address to bind (default: "0.0.0.0")
  -p, --port <number>                  port to listen. if the specified port is not avaiable, lightstatic will find a free port instead (default: "8080")
  -g, --gzip                           gzip encode response content (default: false)
  -o, --open                           open browser window after starting the server (default: false)
  -5, --html5                          use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
  -i, --index <file>                   index file to redirect under html5 mode (default: "index.html")
  -d, --delay <number>                 delay in milliseconds for response (default: "0")
  -s, --store-in-memory                store(cache) static files into memory (default: false)
  -r, --cache-forever-regexp <regexp>  cache file forever if match regexp (default: "")
  -l, --log-dir <directory>            write logs to directory (default: "")
  --base-href <path>                   server base href, useful when under nginx subpath
  --no-access-log                      do not print access log
  --no-color                           disable color log
  -h, --help                           display help for command
````

## Examples

````bash
lightstatic ./dist -5 -o # for local development server
lightstatic -5 -s -r '\w+\.[0-9a-z]{16}\.(js|css|png|svg|jpg)$' -l ./log # for online static  spa server
````

## Middleware

You can use custom middleware by `-m` or `--middleware` option. E.g:

````bash
lightstatic -m ./some_middleware.js
````

The middleware file must export a function which accept three arguments: `req` for http request, `res` for http response and `options` for lightstatic bash command options. 

If the function return `false`, lightstatic won't handle request and response any more.

````js
/** some_middleware.js **/

const { URL } = require('url');
module.exports = async function(req, res, options) {
  if (req.url.startsWith('/__proxy/')) {
    /**
     * const proxy = ...  // do some actual proxy stuff
     * req.pipe(proxy).pipe(res);
     */
    res.end('proxed!');
    return false; // lightstatic won't handle request as middleware function return false
  }
  const u = new URL(req.url, 'http://host');
  const delay = u.searchParams.get('delay');
  if (delay) {
    console.log('delay', delay);
    await new Promise(resolve => setTimeout(resolve, Number(delay)));
  }
  // lightstatic will continue handle request and response
}
````