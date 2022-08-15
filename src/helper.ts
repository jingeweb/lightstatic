import { promises as fs, createReadStream } from 'fs';
import path from 'path';
import { IncomingMessage, ServerResponse } from 'http';
import { createGzip } from 'zlib';
import open from 'open';
import mime from 'mime-types';
import address from 'address';
import { LightStaticOptions } from './common';
import { logger } from './logger';
import DIR_RENDER_TPL from './dir.tpl.html';

function logListening(_ip: string, _port: number, _base: string) {
  logger.info(
    `  http://${_ip}:${_port.toString().green}${_base !== '/' ? ` with baseHref '${_base.toString().yellow}'` : ''}`,
  );
}

export function logStartupInfo(listenPort: number, options: LightStaticOptions) {
  logger.info('Starting up lightstatic, serving: '.yellow + options.root.cyan);
  logger.info('Available on:'.yellow);
  if (options.bind === '0.0.0.0') {
    logListening(address.ip(), listenPort, options.baseHref);
    logListening('127.0.0.1', listenPort, options.baseHref);
  } else {
    logListening(options.bind, listenPort, options.baseHref);
  }
  if (options.html5) {
    logger.info('Html5 route mode rewrite to ' + options.index.cyan);
  }
  logger.info('Press ctrl+c to stop it.');
  if (options.open) {
    open(`http://${options.bind === '0.0.0.0' ? '127.0.0.1' : options.bind}:${listenPort}`);
  }
}

export function sendFile(res: ServerResponse, filepath: string, gzip: boolean) {
  const mimeType = mime.contentType(path.extname(filepath));
  mimeType && res.setHeader('Content-Type', mimeType);
  const stream = createReadStream(filepath, {
    encoding: (mime.charset(path.extname(filepath)) as BufferEncoding) || 'ascii',
  });
  if (gzip) {
    res.setHeader('Content-Encoding', 'gzip');
    const gzipStream = createGzip({
      level: 9,
    });
    stream.pipe(gzipStream);
    gzipStream.pipe(res);
  } else {
    stream.pipe(res);
  }
}

export function send404(req: IncomingMessage, res: ServerResponse, needLogAccess: boolean) {
  res.writeHead(404);
  res.end();
  needLogAccess && logger.access(404, req.url);
}

export async function sendDir(res: ServerResponse, dirpath: string, baseUrl: string, baseHref: string) {
  let _base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  if (baseHref) {
    _base = baseHref + _base;
  }
  const list = (await fs.readdir(dirpath)).map((file) => {
    return `  <li><a href="${_base}${file}">${file}</a></li>`;
  });
  if (baseUrl && baseUrl !== '/') {
    list.unshift(`  <li><a href="${path.dirname(_base)}">../</a></li>`);
  }
  res.end(DIR_RENDER_TPL.replace(/<!--title-->/g, `Index of ${_base}`).replace('<!--files-->', `${list.join('\n')}`));
}
