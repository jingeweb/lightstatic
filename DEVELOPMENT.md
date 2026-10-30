# 开发与发布

本文档面向本仓库的维护者；使用说明见 [README.md](README.md)。

## 发布流程

1. **改版本**：`npm version patch|minor|major`
   - 触发 `scripts/sync-version.mjs`（npm 的 `version` 生命周期）：
     - 同步 `platform/*/package.json` 的 `version`
     - 同步根包 `optionalDependencies`（精确版本，不是 `^`）
     - 同步 `pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude`
   - npm 自动 commit + 打 tag
2. **push**（含 tag）
3. **CI**：`.github/workflows/ci.yml` 矩阵交叉编译 5 个目标 → 各平台 `npm publish --provenance` → 全部成功后 `publish-root` 发根包
4. **刷新 lock**：平台包发布后跑 `pnpm install --lockfile-only`
