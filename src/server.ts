import http, { IncomingMessage, ServerResponse } from 'http';
import path from 'path';
import getPort, { portNumbers } from 'get-port';
import { getStat, resolvePath } from './util';
import { LightStaticOptions } from './common';
import { logger } from './logger';
import { logStartupInfo, send404, sendDir, sendFile } from './helper';
import { FileStore } from './cache';

async function handleFileWithCache(
  req: IncomingMessage,
  res: ServerResponse,
  filepath: string,
  html5: boolean,
  logAccess: boolean,
) {
  let cacheFile = FileStore.getFile(filepath);
  if (!cacheFile) {
    if (!html5) {
      return send404(req, res, logAccess);
    } else {
      cacheFile = FileStore.indexFile;
    }
  }
  const mtime = cacheFile.mtime;
  const ims = req.headers['if-modified-since'];
  if (ims && ims === mtime) {
    res.writeHead(304);
    res.end();
    logAccess && logger.access(304, req.url);
    return;
  }
  res.setHeader('Last-Modified', mtime);
  cacheFile.cacheForever && res.setHeader('Cache-Control', 'max-age=6048000, immutable');
  cacheFile.contentType && res.setHeader('Content-Type', cacheFile.contentType);
  cacheFile.gziped && res.setHeader('Content-Encoding', 'gzip');
  res.setHeader('Content-Length', cacheFile.buffer.length);
  res.write(cacheFile.buffer);
  res.end();
  logAccess && logger.access(200, req.url);
}

async function handle(
  req: IncomingMessage,
  res: ServerResponse,
  rootDir: string,
  middleware: ((req: IncomingMessage, res: ServerResponse) => boolean | undefined | void) | null,
  { delay, baseHref, noAccessLog, html5, gzip, index, storeInMemory, cacheForeverRegexp }: LightStaticOptions,
) {
  if (storeInMemory && req.url === '/' && req.headers['x-reload-cache'] === 'uBLcfD8AalK0402') {
    try {
      await FileStore.readFiles(rootDir, index, cacheForeverRegexp);
      res.end();
    } catch (ex) {
      logger.error(ex);
      res.writeHead(500);
      res.end();
    }
    return;
  }

  if (delay > 0) {
    await new Promise((resolve) => setTimeout(resolve, delay));
  }

  if (middleware && (await middleware(req, res)) === false) {
    logger.info('middleware'.yellow + ': ' + req.url.cyan);
    return;
  }

  let url = req.url;

  const qi = url.indexOf('?');
  if (qi > 0) {
    url = url.substring(0, qi);
  }

  const logAccess = !noAccessLog;

  if (baseHref !== '/') {
    if (url === '/') {
      res.setHeader('Location', baseHref);
      res.writeHead(302);
      res.end();
      logAccess && logger.access(302, req.url);
      return;
    } else if (!url.startsWith(baseHref)) {
      res.writeHead(403);
      // res.end(`request url must starts with base href "${baseHref}"`);
      logAccess && logger.access(403, req.url);
      return;
    } else {
      url = url.substring(baseHref.length - 1);
    }
  }
  let filepath = path.join(rootDir, url);

  if (storeInMemory) {
    await handleFileWithCache(req, res, filepath, html5, logAccess);
    return; // important to return;
  }

  let stat = await getStat(filepath);

  if (stat?.isDirectory() && !html5) {
    await sendDir(res, filepath, url, baseHref === '/' ? '' : baseHref.substring(0, baseHref.length - 1));
    logAccess && logger.access(200, req.url);
    return;
  }
  if (!stat || !stat.isFile()) {
    if (!html5 || /\.\w+$/.test(filepath)) {
      return send404(req, res, logAccess);
    } else {
      // rewrite to index file under html5 route mode
      filepath = path.join(rootDir, index);
      stat = await getStat(filepath);
      if (!stat || !stat.isFile()) {
        return send404(req, res, logAccess);
      }
    }
  }
  const mtime = stat.mtime.toUTCString();
  const ims = req.headers['if-modified-since'];
  if (ims && ims === mtime) {
    res.writeHead(304);
    res.end();
    logAccess && logger.access(304, req.url);
    return;
  }
  res.setHeader('Last-Modified', mtime);
  sendFile(res, filepath, gzip);
  logAccess && logger.access(200, req.url);
}

export async function bootstrap(options: LightStaticOptions) {
  const rootDir = path.resolve(process.cwd(), options.root);
  const freePort = await getPort({
    port: portNumbers(options.port, options.port + 100),
  });

  if (options.storeInMemory) {
    await FileStore.readFiles(rootDir, options.index, options.cacheForeverRegexp);
  }
  const middleware = options.middleware ? require(resolvePath(options.middleware)) : null;
  const logAccess = !options.noAccessLog;

  const server = http.createServer((req, res) => {
    if (req.method !== 'GET') {
      req.destroy(); // 不允许非 GET 请求。
      return;
    }
    handle(req, res, rootDir, middleware, options).catch((err) => {
      logger.error(err);
      if (!res.headersSent) {
        res.writeHead(500);
        res.end();
        logAccess && logger.access(500, req.url);
      }
    });
  });
  server.on('error', (err) => {
    logger.error('server error: ' + err.toString());
  });
  server.listen(freePort, options.bind, () => {
    logStartupInfo(freePort, options);
  });

  process.on('uncaughtException', (err) => {
    logger.error('uncaughtException: ' + err.toString());
    process.exit(-1);
  });
  process.on('unhandledRejection', (err) => {
    logger.error('unhandledRejection: ' + err.toString());
    process.exit(-1);
  });
  process.on('SIGINT', () => {
    logger.info('lightstatic serving stopped'.red);
    process.exit();
  });
}
