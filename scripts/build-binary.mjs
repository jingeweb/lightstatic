/**
 * Build the Rust binary into the platform package layout used by npm:
 *
 *   platform/<os>-<arch>/lightstatic[.exe]
 *
 * CI does the same thing on every runner (see .github/workflows/ci.yml) and
 * publishes each platform folder as its own npm package. Locally this is just
 * a convenience so `node bin/lightstatic.js` works without publishing.
 *
 * Cross builds can override the output folder:
 *   LIGHTSTATIC_PLATFORM=linux LIGHTSTATIC_ARCH=x64 node scripts/build-binary.mjs
 */

import { execFileSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const debug = process.argv.includes('--debug');
const profile = debug ? 'debug' : 'release';

const platform = process.env.LIGHTSTATIC_PLATFORM || process.platform;
const arch = process.env.LIGHTSTATIC_ARCH || process.arch;
const isWindows = platform === 'win32' || platform === 'win';
const exe = isWindows ? 'lightstatic.exe' : 'lightstatic';
// Node 的 Windows 平台名是 "win32"，但目录/npm 包统一用 "win"
const target = `${isWindows ? 'win' : platform}-${arch}`;

const cargoArgs = ['build'];
if (!debug) cargoArgs.push('--release');

process.stdout.write(`[lightstatic] cargo ${cargoArgs.join(' ')}\n`);
execFileSync('cargo', cargoArgs, { cwd: root, stdio: 'inherit' });

const source = path.join(root, 'target', profile, exe);
const destDir = path.join(root, 'platform', target);
const dest = path.join(destDir, exe);

mkdirSync(destDir, { recursive: true });
copyFileSync(source, dest);
if (!isWindows) chmodSync(dest, 0o755);

process.stdout.write(
  `[lightstatic] bundled ${path.relative(root, dest)} (${target}, ${profile})\n`
);
