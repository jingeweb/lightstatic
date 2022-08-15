import { promises as fs } from 'fs';
import path from 'path';
import os from 'os';

export const homeDir = os.homedir();
export const CWD = process.cwd();

export function resolvePath(filepath: string) {
  if (filepath.startsWith('~')) {
    return path.join(homeDir, filepath.substring(1));
  } else {
    return path.resolve(CWD, filepath);
  }
}

export async function getStat(file: string) {
  try {
    return await fs.stat(file);
  } catch (ex) {
    return null;
  }
}

export function getBaseHref(href: string) {
  href = href || '/';
  if (!href.startsWith('/')) {
    href = '/' + href;
  }
  if (!href.endsWith('/')) {
    href += '/';
  }
  return href;
}
