/**
 * Keep every published package on the same version.
 *
 * Runs automatically through the npm `version` lifecycle:
 *
 *   npm version patch     # bump root, sync platform/*, commit + tag
 *
 * It mirrors the root version into:
 *   - platform/<platform>/package.json  ("version")
 *   - root optionalDependencies         ("lightstatic-<platform>": version)
 *   - pnpm-workspace.yaml               (minimumReleaseAgeExclude，否则装不上刚发布的包)
 *
 * pnpm-lock.yaml 不在这里刷新：新版本平台包要等 CI 发布后才存在，
 * 届时手动跑一次 `pnpm install --lockfile-only` 即可（脚本结束会提示）。
 */

import { existsSync } from 'node:fs';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const rootPkgPath = path.join(root, 'package.json');
const platformDir = path.join(root, 'platform');

const rootPkg = JSON.parse(await readFile(rootPkgPath, 'utf8'));
const version = rootPkg.version;

rootPkg.optionalDependencies = rootPkg.optionalDependencies || {};

const entries = await readdir(platformDir, { withFileTypes: true });
const platforms = [];

for (const entry of entries) {
  if (!entry.isDirectory()) continue;

  const pkgPath = path.join(platformDir, entry.name, 'package.json');
  let pkg;
  try {
    pkg = JSON.parse(await readFile(pkgPath, 'utf8'));
  } catch {
    continue;
  }

  platforms.push(pkg.name);

  if (pkg.version !== version) {
    pkg.version = version;
    await writeFile(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
  }

  if (rootPkg.optionalDependencies[pkg.name] !== version) {
    rootPkg.optionalDependencies[pkg.name] = version;
  }
}

// Drop optionalDependencies that no longer have a platform folder.
for (const name of Object.keys(rootPkg.optionalDependencies)) {
  if (name.startsWith('lightstatic-') && !platforms.includes(name)) {
    delete rootPkg.optionalDependencies[name];
  }
}

rootPkg.optionalDependencies = Object.fromEntries(
  Object.entries(rootPkg.optionalDependencies).sort(([a], [b]) =>
    a.localeCompare(b)
  )
);

await writeFile(rootPkgPath, `${JSON.stringify(rootPkg, null, 2)}\n`);

// pnpm 默认会拒绝安装"太新"的包（minimumReleaseAge），刚发版的平台包必须显式放行。
const workspaceFile = path.join(root, 'pnpm-workspace.yaml');
if (existsSync(workspaceFile)) {
  const block = `minimumReleaseAgeExclude:\n${platforms
    .sort()
    .map((name) => `  - ${name}@${version}`)
    .join('\n')}\n`;
  const text = await readFile(workspaceFile, 'utf8');
  const next = /^minimumReleaseAgeExclude:/m.test(text)
    ? text.replace(/^minimumReleaseAgeExclude:[\s\S]*?(?=^\S|\Z)/m, block)
    : `${text}${text.endsWith('\n') ? '' : '\n'}${block}`;
  if (next !== text) {
    await writeFile(workspaceFile, next);
  }
}

const lockFile = path.join(root, 'pnpm-lock.yaml');
if (existsSync(lockFile)) {
  process.stdout.write(
    '[lightstatic] 平台包发布后记得刷新 lock：pnpm install --lockfile-only\n'
  );
}

process.stdout.write(
  `[lightstatic] synced ${platforms.length} platform package(s) to ${version}\n`
);
