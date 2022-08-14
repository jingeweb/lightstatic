import { promises as fs } from 'fs';
import path from 'path';
import { gzip } from 'zlib';
import mime from 'mime-types';
import { logger } from './logger';

interface File {
  buffer: Buffer;
  gziped: boolean;
  contentType: string | false;
  mtime: string;
  cacheForever: boolean;
}

export class FileCacheStore {
  #store: Map<string, File>;
  indexFile: File;
  cacheForeverRegexp: RegExp;

  async #loopScan(store: Map<string, File>, dir: string) {
    const files = await fs.readdir(dir);

    for await (const file of files) {
      const filepath = path.join(dir, file);
      try {
        const stat = await fs.stat(filepath);
        if (stat.isDirectory()) {
          await this.#loopScan(store, path.join(dir, file));
        } else if (stat.isFile()) {
          if (stat.size >= 5 * 1024 * 1024) {
            logger.error('ignore cache file: ' + filepath + ' due to size is larger than 5m');
            continue;
          }
          const buffer = await fs.readFile(filepath);
          const gzipBuf = await new Promise<Buffer>((resolve, reject) =>
            gzip(buffer, (err, result) => {
              if (err) reject(err);
              else resolve(result);
            }),
          );
          const gziped = gzipBuf.length < buffer.length;
          store.set(filepath, {
            gziped,
            mtime: stat.mtime.toUTCString(),
            contentType: mime.contentType(path.extname(filepath)),
            buffer: gziped ? gzipBuf : buffer,
            cacheForever: this.cacheForeverRegexp?.test(filepath),
          });
          // console.log(store.get(filepath));
        }
      } catch (ex) {
        logger.error('ingore cache file: ' + filepath + ' due to: ' + ex.toString());
      }
    }
  }
  async readFiles(root: string, indexFile: string, cacheForeverRegexp: string) {
    this.cacheForeverRegexp = cacheForeverRegexp ? new RegExp(cacheForeverRegexp) : null;
    const store: Map<string, File> = new Map();
    await this.#loopScan(store, root);
    this.#store?.clear(); // clear to gc
    this.#store = store;
    this.indexFile = store.get(path.join(root, indexFile));
    logger.info('Files stored into memory.');
  }

  getFile(filepath: string) {
    return this.#store.get(filepath);
  }
}

export const FileStore = new FileCacheStore();
