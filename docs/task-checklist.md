# Coffee ERP 任务清单

本文件记录当前提交中已经完成和待推进的里程碑任务。详细验收标准以 `docs/milestones.md` 为准，测试写法以 `docs/testing-guidelines.md` 为准。

## 当前状态

- [x] M0：项目骨架与工程基线。
- [x] M1：领域模型与 Seed Data。
- [x] M2：冲煮匹配与参数拟合。
- [x] M3：本地状态与 Cloudflare API。
- [x] M4：资料维护界面。
- [x] M5：入库与今日推荐。
- [x] M6：误操作恢复与移动端体验。
- [x] M7：部署与发布配置。

## M0 已完成

- [x] 初始化 Dioxus Web 依赖、`Dioxus.toml` 和基础 `main.rs`。
- [x] 建立 `src/domain`、`src/storage`、`src/ui` 目录边界。
- [x] 建立 `src/lib.rs`，让领域层、存储层和 UI 层可被测试引用。
- [x] 建立移动端 UI shell，包含今日、入库、资料、设置四个入口。
- [x] 建立移动端底部导航和页面容器。
- [x] 建立 `worker/` 目录和 Cloudflare API 占位文件。
- [x] 配置 `.gitignore`、`Cargo.toml`、`Cargo.lock`、`Makefile`。

## M1 已完成

- [x] 实现 `Store`、`CoffeeParameters`、`CatalogOption`、`RoastLevelOption`。
- [x] 实现 `CoffeeBean`、`RoastMethod`、`RoastProfile`、`ProductLine`。
- [x] 实现 `GrinderProfile`、`BrewingPlanCategory`、`BrewingPlan`、`BrewingMatchAttribute`。
- [x] 实现 `BrewingPlanParameters`、`BrewingAgeFitting`、`WaterQualityAdjustment`、`BrewRatio`。
- [x] 实现 `RoastBatch`、`BatchStatus`、`AppState`。
- [x] 内置豆种、烘焙度、处理法、Ditting、冲煮方案、水质修正规则。
- [x] 将 `少厌氧` 统一为 `轻厌氧`，并保留 `强厌氧`。
- [x] 实现批次编号生成：`yyyyMMdd-batch_code-daily_sequence`。
- [x] 实现基础数据校验函数。
- [x] 覆盖 seed data、JSON roundtrip、批次编号、校验失败场景。

## M2 已完成

- [x] 实现批次上下文解析：批次 -> 烘焙品类 -> 咖啡豆。
- [x] 实现冲煮方案 OR 匹配逻辑。
- [x] 实现匹配排序：匹配属性数量更少优先，再按 priority 和顺序。
- [x] 实现养豆天数计算。
- [x] 实现 day 0 到 day 14 线性拟合。
- [x] 实现 TDS 水质修正，边界归入较低区间。
- [x] 实现粉量 0.1g 归一和总水量计算。
- [x] 输出 UI 可直接展示的 `BrewingRecommendation`。
- [x] 覆盖 OR 匹配、排序、无匹配、天数拟合、TDS 边界、总水量、DTO 场景。

## M3 已完成

- [x] 实现 Worker API：`GET /api/state?store_id=...`。
- [x] 实现 Worker API：`PUT /api/state?store_id=...`。
- [x] 使用 KV key：`coffee_erp:store:{store_id}:state`。
- [x] GET 不存在状态时返回 seed state。
- [x] PUT 校验 revision，一致才写入 KV，并递增 revision。
- [x] CORS 使用 `ALLOWED_ORIGINS`。
- [x] 前端 API client 使用 `PUBLIC_API_BASE_URL`。
- [x] IndexedDB 缓存当前 AppState 的 trait 边界。
- [x] localStorage 保存 `store_id` 和 UI 偏好的 trait 边界。
- [x] revision 冲突时阻止保存，并提示用户刷新远端最新数据；MVP 不做 diff。
- [x] 补 Worker 和前端 storage 测试。

## M4 已完成

- [x] 参数目录维护：豆种、烘焙度、处理法。
- [x] 咖啡豆维护：名称、豆种、处理法、产地、备注。
- [x] 烘焙方法维护。
- [x] 烘焙品类维护：豆子、方法、烘焙度、产品线、`batch_code`。
- [x] `batch_code` 自动建议，并允许保存前修改。
- [x] 冲煮方案分类维护。
- [x] 冲煮方案维护：匹配属性、注水段数、滤杯、磨豆机、比例、day 0/day 14、说明文字。
- [x] 归档操作接入 pending undo（5 秒倒计时，超时自动提交）。
- [x] 表单校验错误展示。
- [x] 统一 AppState 保存入口：`save_app_state` 异步函数，资料编辑成功后自动调用。
- [x] pending undo 期间普通写操作按钮禁用；当前 pending 的“确认归档”和“撤销”作为唯一例外。
- [x] 消除 `src/ui/mod.rs` 整表 `.clone()`，改用 `iter()` 迭代。

## M5 待推进

- [x] 入库页面选择烘焙品类。
- [x] 自动生成批次编号。
- [x] 默认 100g 批次容量。
- [x] 保存批次并记录烘焙时间。
- [x] 今日推荐按批次显示养豆天数。
- [x] 手冲批次显示全部匹配冲煮方案。
- [x] 意式批次显示萃取备注，不显示手冲拟合。
- [x] 默认粉量 16g，允许 0.1g 微调。
- [x] 按门店 TDS 套用水质修正。

## M6 待推进

- [x] 危险操作进入 5 秒 pending undo。
- [x] pending 期间禁用其他危险操作。
- [x] 5 秒内允许撤销，超时后提交。
- [x] 保存失败时恢复 pending 前状态。
- [x] 移动端错误提示和空状态。
- [x] 页面加载、保存中、冲突刷新状态。

## M7 待推进

- [x] Cloudflare Pages / Workers / KV 配置文档。
- [x] EdgeOne Pages 静态前端镜像配置文档。
- [x] 环境变量说明：`PUBLIC_API_BASE_URL`、`ALLOWED_ORIGINS`、`KV_NAMESPACE`、`STORE_ID_SEED`。
- [x] 本地构建和部署命令整理。
- [x] 发布前检查清单。

## 当前验证

- [x] `cargo fmt --check`
- [x] `cargo test`
- [x] `cargo check --all-targets`
- [x] `dx build --platform web`
- [x] `pnpm test` in `worker/`
- [x] `pnpm typecheck` in `worker/`
- [x] 源码中未使用 `allow(dead_code)`、`todo!()`、`unimplemented!()`、显式 `panic!`、`structuredClone`、`deep_clone`、`deepClone` 绕过验收。
