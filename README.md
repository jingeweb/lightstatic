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
  -V, --version            output the version number
  -b, --bind <string>      ip address to bind (default: "0.0.0.0")
  -p, --port <number>      port to listen. if the specified port is not avaiable, sfserver will find a free port instead (default: 8080)
  -g, --gzip               gzip encode response content (default: false)
  -o, --open               open browser window after starting the server (default: false)
  -5, --html5              use html5 mode url route(history api fallback like webpack-dev-server) (default: false)
  -i, --index <file>       index file to redirect under html5 mode (default: "index.html")
  -d, --delay <number>     delay in milliseconds for response (default: 0)
  -m, --middleware <file>  middleware file to use
  --base-href <path>       server base href, useful when under nginx subpath
  --no-access              do not print access log
  --no-log                 do not print any log expect errors
  --no-color               disable color log
  -h, --help               display help for command
````

## Examples

````bash
lightstatic
lightstatic ./dist -g -5 -o
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