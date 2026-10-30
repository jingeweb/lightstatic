#!/usr/bin/env node
'use strict';

/**
 * Thin launcher that finds a platform specific `lightstatic` binary and
 * hands the process over to it (same argv, same stdio, same exit code).
 *
 * Lookup order:
 *   1. $LIGHTSTATIC_BINARY                    (explicit override)
 *   2. optional dependency lightstatic-<os>-<arch>
 *   3. platform/<os>-<arch>/lightstatic       (running from the repo)
 *   4. target/{release,debug}/lightstatic     (running from the repo)
 *
 * The published package ships no binary itself: npm installs only the
 * matching `lightstatic-<platform>-<arch>` package through optionalDependencies,
 * and this script spawns the binary from there.
 */

const { spawn } = require('node:child_process');
const { accessSync, chmodSync, constants, statSync } = require('node:fs');
const path = require('node:path');

const PLATFORM = process.platform;
const ARCH = process.arch;
const IS_WINDOWS = PLATFORM === 'win32';
const EXE = IS_WINDOWS ? 'lightstatic.exe' : 'lightstatic';
/**
 * npm 包名和 platform/ 目录统一用 `<os>-<arch>`：
 * Node 里 Windows 的 process.platform 是 "win32"，但包名用 "win"
 * （lightstatic-win32-x64 这个名字在 npm 上不可用）。
 */
const TARGET = `${IS_WINDOWS ? 'win' : PLATFORM}-${ARCH}`;

function isExecutable(file) {
  try {
    if (!statSync(file).isFile()) return false;
    if (PLATFORM !== 'win32') accessSync(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** npm occasionally loses the executable bit, restore it before giving up. */
function ensureExecutable(file) {
  if (isExecutable(file)) return true;
  if (PLATFORM === 'win32') return false;
  try {
    chmodSync(file, 0o755);
  } catch {
    return false;
  }
  return isExecutable(file);
}

function fromOptionalPackage() {
  const pkg = `lightstatic-${TARGET}`;
  try {
    return path.join(
      path.dirname(require.resolve(`${pkg}/package.json`)),
      EXE
    );
  } catch {
    return null;
  }
}

const candidates = [
  process.env.LIGHTSTATIC_BINARY,
  fromOptionalPackage(),
  path.join(__dirname, '..', 'platform', TARGET, EXE),
  path.join(__dirname, '..', 'target', 'release', EXE),
  path.join(__dirname, '..', 'target', 'debug', EXE),
].filter(Boolean);

const binary = candidates.find(ensureExecutable);

if (!binary) {
  process.stderr.write(
    [
      `lightstatic: no prebuilt binary for ${TARGET} was found.`,
      '',
      'Looked in:',
      ...candidates.map((c) => `  - ${c}`),
      '',
      'How to fix:',
      `  - reinstall the matching optional package: npm i lightstatic-${TARGET}`,
      '  - or install from source:                 cargo install lightstatic',
      '  - or download a release:                  https://github.com/jingeweb/lightstatic/releases',
      '  - or point $LIGHTSTATIC_BINARY at an existing binary',
      '',
    ].join('\n')
  );
  process.exit(1);
}

const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });

child.on('error', (err) => {
  process.stderr.write(`lightstatic: failed to launch ${binary}: ${err.message}\n`);
  process.exit(1);
});

for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
  process.on(signal, () => {
    try {
      child.kill(signal);
    } catch {
      /* the child is already gone */
    }
  });
}

child.on('exit', (code, signal) => {
  if (signal) {
    // Mirror the child's death signal so shells see the real exit status.
    process.kill(process.pid, signal);
  } else {
    process.exit(code === null ? 1 : code);
  }
});
