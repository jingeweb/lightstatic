# 开发与发布

本文档面向本仓库的维护者；使用说明见 [README.md](README.md)。

## 发布流程

1. **改版本**：`npm version patch|minor|major`
   - 触发 `scripts/sync-version.mjs`（npm 的 `version` 生命周期）：
     - 同步 `platform/*/package.json` 的 `version`
     - 同步根包 `optionalDependencies`（精确版本，不是 `^`）
     - 同步 `pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude`
     - 同步 `Cargo.toml` / `Cargo.lock` 的 `[package].version`
   - npm 自动 commit + 打 tag
2. **push**（含 tag）
3. **CI**：`.github/workflows/ci.yml` 矩阵交叉编译 5 个目标 → 各平台 `npm publish --provenance` → 全部成功后 `publish-root` 发根包
4. **刷新 lock**：平台包发布后跑 `pnpm install --lockfile-only`


## 断点续传

文件响应支持单个 HTTP 字节范围：`Range: bytes=100-199`、`bytes=100-`、`bytes=-100`。
有效范围返回 `206 Partial Content`，附带 `Content-Range` 和准确的 `Content-Length`；
无法满足的范围返回 `416`。多范围请求、不支持的范围单位和非法语法会被忽略，返回完整文件。

支持用响应的 ETag 作为 `If-Range` 验证器；不匹配时返回完整文件，避免拼接不同版本。
弱 ETag 和日期形式的 `If-Range` 同样回退完整响应。HEAD 请求忽略 Range。

启用 `-g` 且客户端接受 gzip 时，服务器忽略 Range，返回完整 gzip 响应（`Accept-Ranges: none`）。
续传客户端可发送 `Accept-Encoding: identity` 使用未压缩的字节范围：

```bash
curl -C - -H 'Accept-Encoding: identity' -o archive.zip http://localhost:8080/archive.zip
```
