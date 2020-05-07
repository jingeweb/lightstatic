const http = require('http');
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const getPort = require('get-port');
const colors = require('colors');
const address = require('address');
const mime = require('mime');
const open = require('open');
const _util = require('./util');

const DIR_RENDER_TPL = fs.readFileSync(path.join(__dirname, 'dir.tpl.html'), 'utf-8');

function logAccess(status, url) {
  console.log('GET'.yellow, status < 400 ? status.toString().green : status.toString().red, url.cyan);
}

function sendFile(res, filepath, gzip) {
  res.setHeader('Content-Type', mime.getType(path.extname(filepath)) + '; charset=utf-8');
  let stream = fs.createReadStream(filepath, {
    encoding: 'utf-8'
  });
  if (gzip) {
    res.setHeader('Content-Encoding', 'gzip');
    const gzipStream = zlib.createGzip({
      level: 9
    });
    stream.pipe(gzipStream);
    stream = gzipStream;
  }
  stream.pipe(res);
}

function send404(req, res, options) {
  res.writeHead(404);
  res.end();
  options.access && logAccess(404, req.url);
}

async function sendDir(res, dirpath, baseUrl) {
  const _base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  const list = (await fs.promises.readdir(dirpath)).map(file => {
    return `  <li><a href="${_base}${file}">${file}</a></li>`;
  });
  if (baseUrl && baseUrl !== '/') {
    list.unshift(`  <li><a href="${path.dirname(_base)}">../</a></li>`);
  }
  res.write(
    DIR_RENDER_TPL
      .replace(/<!--title-->/g, `Index of ${_base}`)
      .replace('<!--files-->', `${list.join('\n')}`)
  );
}

function bootstrap(options) {
  if (options.noColor) {
    colors.disable();
  }

  // TODO: implement https support 
  // const cert = options.cert ? resolvePath(options.cert) : null;
  const root = _util.resolvePath(options.root);
  const baseHref = _util.getBaseHref(options.baseHref);

  async function handle(req, res) {
    let url = req.url;
    if (baseHref !== '/') {
      if (url === '/') {
        res.setHeader('Location', baseHref);
        res.writeHead(302);
        res.end();
        options.access && logAccess(302, req.url);
        return;
      } else if (!url.startsWith(baseHref)) {
        res.writeHead(403);
        res.end('base href not match.');
        options.access && logAccess(403, req.url);
        return;
      } else {
        url = url.substring(baseHref.length - 1);
      }
    }

    let filepath = path.join(root, url);
    let stat = await _util.getStat(filepath);
    if (stat && stat.isDirectory() && !options.html5) {
      await sendDir(res, filepath, req.url);
      options.access && logAccess(200, req.url);
      return;
    }
    if (!stat || !stat.isFile()) {
      if (!options.html5 || /\.\w+$/.test(filepath)) {
        return send404(req, res, options);
      } else {
        // rewrite to index file under html5 route mode
        filepath = path.join(root, options.index);
        stat = await _util.getStat(filepath);
        if (!stat || !stat.isFile()) {
          return send404(req, res, options);
        }
      }
    }
    const mtime = stat.mtime.toUTCString();
    const ims = req.headers['if-modified-since'];
    if (ims && ims === mtime) {
      res.writeHead(304);
      res.end();
      options.access && logAccess(304, req.url);
      return;
    }
    res.setHeader('Last-Modified', mtime);
    sendFile(res, filepath, options.gzip);
    options.access && logAccess(200, req.url);
  }

  const server = http.createServer((req, res) => {
    if (req.method !== 'GET') {
      res.writeHead(405);
      res.end('http method not allowd, only accept GET method.');
      options.access && logAccess(405, req.url);
      return;
    }
    handle(req, res).catch(err => {
      console.error(err);
      if (!res.headersSent) {
        res.writeHead(500);
        res.end();
        options.access && logAccess(500, req.url);
      }
    });
  });
  server.on('error', err => {
    console.error(err.toString());
  });
  getPort({
    port: getPort.makeRange(options.port, options.port + 100)
  }).then(freePort => {
    server.listen(freePort, options.host, () => {
      if (!options.log) {
        return;
      }
      function _log(_ip, _port, _base) {
        console.log(`  http://${_ip}:${_port.toString().green}${_base !== '/' ? ` with baseHref '${_base.toString().yellow}'` : ''}`);
      }
      console.log('Starting up lightstatic, serving:'.yellow, options.root.cyan);
      console.log('Available on:'.yellow);
      if (options.bind === '0.0.0.0') {
        _log(address.ip(), freePort, baseHref);
        _log('127.0.0.1', freePort, baseHref);
      } else {
        _log(options.bind, freePort, baseHref);
      }
      if (options.html5) {
        console.log('Html5 route mode rewrite to', options.index.cyan);
      }
      console.log('Press ctrl+c to stop it.');
      if (options.open) {
        open(`http://${options.bind === '0.0.0.0' ? '127.0.0.1' : options.bind}:${freePort}`);
      }
    });
  });

  options.log && process.on('SIGINT', () => {
    console.log('sfserver stopped'.red);
    process.exit();
  });
}

module.exports = {
  bootstrap
};
