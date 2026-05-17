# Coffee ERP 部署与发布说明（M7）

本文档记录当前 Coffee ERP 的线上拓扑、发布命令和验收步骤。M7 的目标是：Cloudflare 作为主 API 与主存储，Cloudflare Pages / EdgeOne Pages 只承载前端静态资源。

## 1. 当前线上环境

| 类型 | 当前值 | 说明 |
| --- | --- | --- |
| Cloudflare Worker API | `https://coffee-erp-api.581006.xyz` | 前端唯一 API 基地址，绑定在 Cloudflare zone `581006.xyz` 下 |
| Worker 默认域名 | `https://coffee-erp-worker.gdindex.workers.dev` | 仅作为回退和排障入口，不作为前端默认配置 |
| Cloudflare Pages 主站 | `https://coffee-erp.pages.dev` | 海外主静态站 |
| Cloudflare Pages 自定义域名 | `https://coffee-erp.581006.xyz` | 绑定到 Cloudflare Pages 项目 `coffee-erp` |
| Cloudflare Pages 当前预览 | `https://5e6f6b26.coffee-erp.pages.dev` | 最近一次手动发布产生的预览地址 |
| EdgeOne Pages 主站 | `https://coffee-erp.cic.cab` | 中国大陆访问优先入口，仅托管静态资源 |
| EdgeOne 项目 ID | `pages-6cfhtul7ttt7` | EdgeOne Pages 项目 |
| Cloudflare KV namespace ID | `55de722cbd614f9c9814eb791a528f0c` | Worker 生产 KV 绑定目标 |

不要把 token、账号密钥或 `.edgeone` 本地登录信息提交到仓库。`worker/wrangler.toml` 和 `.edgeone/*` 应保持在 `.gitignore` 中。

## 2. 架构边界

- Cloudflare Worker 承载 API：`GET /api/state`、`PUT /api/state`。
- Cloudflare KV 是唯一主存储。
- Cloudflare Pages 承载一份前端静态资源。
- EdgeOne Pages 承载同一份前端静态资源，服务中国大陆访问。
- EdgeOne 不承载 API，不承载 KV。前端始终通过 `PUBLIC_API_BASE_URL` 访问 Cloudflare Worker API。

## 3. 前置条件

本地需要：

- `cargo`
- `dx`（Dioxus CLI）
- `pnpm`
- `node`
- `edgeone` CLI，版本不低于 `1.2.30`

登录检查：

```bash
pnpm dlx wrangler whoami
edgeone whoami
```

EdgeOne 使用 China 站点账号。若需要重新登录，使用：

```bash
edgeone login --site china
```

## 4. 本地部署配置

### 4.1 Worker 配置

复制模板：

```bash
cp worker/wrangler.toml.example worker/wrangler.toml
```

`worker/wrangler.toml` 至少需要包含：

```toml
name = "coffee-erp-worker"
main = "src/index.ts"
compatibility_date = "2025-04-01"

[[kv_namespaces]]
binding = "KV_NAMESPACE"
id = "55de722cbd614f9c9814eb791a528f0c"

[vars]
STORE_ID_SEED = "store-default"
ALLOWED_ORIGINS = "https://coffee-erp.pages.dev,https://coffee-erp.581006.xyz,https://5e6f6b26.coffee-erp.pages.dev,https://coffee-erp.cic.cab"
```

如果保留旧预览域名，也可以临时把旧预览加入 `ALLOWED_ORIGINS`。确认不再使用后应移除，避免 CORS 白名单长期膨胀。

### 4.2 前端 API 基址

正式构建统一使用：

```bash
PUBLIC_API_BASE_URL="https://coffee-erp-api.581006.xyz"
```

## 5. 发布流程

### 5.1 部署 Worker API

在 `worker/` 目录执行：

```bash
pnpm dlx wrangler deploy --domain coffee-erp-api.581006.xyz
```

预期：

- Worker 部署成功。
- Custom Domain 指向 `coffee-erp-api.581006.xyz`。
- `GET https://coffee-erp-api.581006.xyz/api/state?store_id=...` 可访问。

### 5.2 构建前端

在仓库根目录执行 release 构建：

```bash
PUBLIC_API_BASE_URL="https://coffee-erp-api.581006.xyz" dx build --platform web --release
```

发布目录：

```text
target/dx/coffee-erp/release/web/public
```

如果曾经用其他 API 域名构建过，建议先清理旧产物目录再构建，避免旧 hash 资源残留：

```bash
rm -rf target/dx/coffee-erp/release/web/public
PUBLIC_API_BASE_URL="https://coffee-erp-api.581006.xyz" dx build --platform web --release
```

注意：Dioxus release 构建期间 `wasm-opt` 可能打印 DWARF/SIGABRT 相关信息，只要最终出现 `Client build completed successfully`，且 `public/assets/*.wasm` 生成成功，即可继续发布。

### 5.3 部署 Cloudflare Pages

在仓库根目录执行：

```bash
pnpm dlx wrangler pages deploy target/dx/coffee-erp/release/web/public --project-name coffee-erp --commit-dirty=true
```

预期：

- `https://coffee-erp.pages.dev` 更新。
- 命令输出新的预览 URL，例如 `https://5e6f6b26.coffee-erp.pages.dev`。
- 如果预览 URL 需要访问 API，必须把该预览域名加入 Worker `ALLOWED_ORIGINS` 并重新部署 Worker。

### 5.4 部署 EdgeOne Pages

在仓库根目录执行：

```bash
edgeone pages deploy target/dx/coffee-erp/release/web/public -n coffee-erp
```

预期：

- `EDGEONE_DEPLOY_TYPE=custom`
- `EDGEONE_DEPLOY_URL=https://coffee-erp.cic.cab`
- EdgeOne 控制台中项目 ID 为 `pages-6cfhtul7ttt7`

如果输出的是 preset URL 且带查询参数，必须完整保留整个 URL，不能截断 `?` 后面的参数。

## 6. Smoke Test

### 6.1 API CORS 与状态读写

可使用下面脚本验证允许来源、读写成功和 revision 冲突：

```bash
node - <<'NODE'
const api = "https://coffee-erp-api.581006.xyz";
const origin = "https://coffee-erp.cic.cab";
const store = `smoke-${Date.now()}`;

const headers = { Origin: origin };
const first = await fetch(`${api}/api/state?store_id=${store}`, { headers });
console.log("GET", first.status, first.headers.get("access-control-allow-origin"));
const payload = await first.json();

const putHeaders = {
  Origin: origin,
  "Content-Type": "application/json",
};
const saved = await fetch(`${api}/api/state?store_id=${store}`, {
  method: "PUT",
  headers: putHeaders,
  body: JSON.stringify(payload.state),
});
console.log("PUT", saved.status);

const conflict = await fetch(`${api}/api/state?store_id=${store}`, {
  method: "PUT",
  headers: putHeaders,
  body: JSON.stringify(payload.state),
});
console.log("STALE_PUT", conflict.status, await conflict.text());
NODE
```

通过标准：

- `GET 200`。
- `Access-Control-Allow-Origin` 等于发起请求的 Origin。
- 首次 `PUT 200`。
- 重放旧 revision 的 `PUT 409`，错误码为 `revision_conflict`。

### 6.2 前端静态站

验证四个入口都可访问：

```bash
curl -I https://coffee-erp.pages.dev
curl -I https://coffee-erp.581006.xyz
curl -I https://5e6f6b26.coffee-erp.pages.dev
curl -I https://coffee-erp.cic.cab
```

通过标准：

- HTTP 状态为 `200` 或缓存命中的 `304`。
- 浏览器加载后没有 CORS 错误。
- Network 中 `/api/state?...` 请求指向 `https://coffee-erp-api.581006.xyz`。

### 6.3 确认前端产物使用新 API 域名

发布后可以抽查 wasm 资源中是否仍残留旧 API 域名：

```bash
strings target/dx/coffee-erp/release/web/public/assets/*.wasm | rg "coffee-erp-api|coffee-erp-worker|localhost:8787"
```

通过标准：

- 应包含 `https://coffee-erp-api.581006.xyz`。
- 不应包含 `https://coffee-erp-worker.gdindex.workers.dev`。
- `localhost:8787` 可能来自开发 fallback 代码，不代表生产 API 被错误配置。

## 7. 发布前检查清单

- [ ] Worker 已通过 `pnpm dlx wrangler deploy --domain coffee-erp-api.581006.xyz` 部署。
- [ ] Worker `ALLOWED_ORIGINS` 包含 `https://coffee-erp.pages.dev`、`https://coffee-erp.581006.xyz`、当前 Cloudflare Pages 预览域名和 `https://coffee-erp.cic.cab`。
- [ ] 前端 release 构建时 `PUBLIC_API_BASE_URL=https://coffee-erp-api.581006.xyz`。
- [ ] Cloudflare Pages 已重新部署 `target/dx/coffee-erp/release/web/public`。
- [ ] EdgeOne Pages 已重新部署同一份 `target/dx/coffee-erp/release/web/public`。
- [ ] API smoke test 通过 `GET 200 / PUT 200 / stale PUT 409`。
- [ ] 四个前端入口均返回 `200` 或 `304`。
- [ ] wasm 产物不再包含旧 Worker API 域名。

## 8. 回滚与故障排查

### 8.1 回滚路径

- Cloudflare Pages：在 Pages 控制台选择上一个成功部署并回滚，或重新部署上一版前端构建产物目录。
- Cloudflare Worker：重新部署上一版 `worker/src/index.ts` 和对应 `worker/wrangler.toml`。
- EdgeOne Pages：在 EdgeOne 控制台回滚到上一个成功部署，或重新发布上一版静态目录。

### 8.2 常见故障定位

1. 前端请求仍打到旧域名：
   - 重新清理 `target/dx/coffee-erp/release/web/public`，带 `PUBLIC_API_BASE_URL` 重新 release 构建，再重新部署两个 Pages。
2. EdgeOne 页面可打开但 API 失败：
   - 检查 Worker `ALLOWED_ORIGINS` 是否包含 `https://coffee-erp.cic.cab`。
3. Cloudflare Pages 预览 API 失败：
   - 把新的预览域名加入 `ALLOWED_ORIGINS`，重新部署 Worker。
4. API 能访问但数据不持久：
   - 检查 `KV_NAMESPACE` 绑定名是否为 `KV_NAMESPACE`，namespace ID 是否为当前生产 ID。
5. PUT 总是冲突：
   - 确认 PUT body 使用的是最新 GET 返回的 `revision`。
