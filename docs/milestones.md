# Coffee ERP 实施里程碑

## 目标

按 MVP 范围推进代码实现：

- 单店使用。
- Cloudflare KV 作为主存储。
- IndexedDB 作为浏览器本地缓存。
- 基本入库、养豆计时、冲煮方案拟合。
- Cloudflare Pages 主站，EdgeOne Pages 前端静态镜像。

每个里程碑必须满足：

- 有明确的代码交付物。
- 有测试覆盖目标。
- 有客观验收标准。
- 不把后续功能混入当前阶段。
- 测试用例必须符合 `docs/testing-guidelines.md`。

当前执行状态和可勾选任务清单见 `docs/task-checklist.md`。

## M0：项目骨架与工程基线

### 目标

将当前空 Rust 项目整理为可运行的 Dioxus Web 应用，并建立前端、领域层、存储层、Worker API 的目录边界。

### 任务包

- M0.1 初始化 Dioxus Web 依赖、`Dioxus.toml` 和基础 `main.rs`。
- M0.2 建立 `src/domain`、`src/storage`、`src/ui` 目录和 `mod.rs`。
- M0.3 建立 UI shell：今日、入库、资料、设置四个入口。
- M0.4 建立移动端底部导航和页面容器。
- M0.5 建立 `worker/` 目录和 Cloudflare API 占位文件。
- M0.6 配置 `.gitignore`、格式化和检查命令。

### 测试覆盖

- 暂不要求业务单元测试。
- 必须运行：
  - `cargo check`
  - `cargo fmt --check`

### 客观验收

- `cargo check` 通过。
- `cargo fmt --check` 通过。
- 本地启动后能看到四个页面入口。
- 页面切换不报错。
- 目录中存在：
  - `src/domain/mod.rs`
  - `src/storage/mod.rs`
  - `src/ui/mod.rs`
  - `worker/`

### 不包含

- 不实现业务模型。
- 不实现 KV API。
- 不实现 IndexedDB。

## M1：领域模型与 Seed Data

### 目标

定义 MVP 领域模型，内置商家提供的 seed data，并完成基础校验和批次编号生成。

### 任务包

- M1.1 实现模型：`Store`、`CoffeeParameters`、`CatalogOption`、`RoastLevelOption`。
- M1.2 实现模型：`CoffeeBean`、`RoastMethod`、`RoastProfile`、`ProductLine`。
- M1.3 实现模型：`GrinderProfile`、`BrewingPlanCategory`、`BrewingPlan`、`BrewingMatchAttribute`。
- M1.4 实现模型：`BrewingPlanParameters`、`BrewingAgeFitting`、`WaterQualityAdjustment`、`BrewRatio`。
- M1.5 实现模型：`RoastBatch`、`BatchStatus`、`AppState`。
- M1.6 实现 seed data：
  - 豆种。
  - 烘焙度。
  - 处理法。
  - Ditting 磨豆机。
  - 冲煮方案。
  - 水质修正规则。
- M1.7 实现批次编号生成：`yyyyMMdd-batch_code-daily_sequence`。
- M1.8 实现基础校验函数。

### 测试覆盖

必须覆盖以下函数或等价函数：

- `seed_app_state()`
  - 检查豆种数量、处理法包含 `轻厌氧` 和 `强厌氧`。
  - 检查只初始化一个 Ditting 磨豆机。
  - 检查冲煮方案分类和方案数量正确。
- `generate_batch_no(date, batch_code, existing_batches)`
  - 同一天连续编号。
  - 不同日期从 001 开始。
  - 不同 `batch_code` 仍使用当天全局递增序号。
- `validate_app_state(state)`
  - 空名称失败。
  - `batch_code` 为空失败。
  - 被引用目录项不存在失败。
- JSON roundtrip
  - `AppState` 序列化后反序列化保持关键字段一致。

### 客观验收

- 所有模型实现 `Serialize` / `Deserialize`。
- `seed_app_state()` 生成的 `schema_version` 为当前版本。
- seed data 中没有 `少厌氧`，只有 `轻厌氧`。
- 批次编号示例必须能生成：
  - `20260502-YJPO-001`
  - `20260502-YJPO-002`
  - `20260502-ESP-003`
- `cargo test` 通过。
- `cargo check` 通过。

### 不包含

- 不实现 UI 编辑页面。
- 不实现远端 API。
- 不实现 IndexedDB。

## M2：冲煮匹配与参数拟合

### 目标

实现从批次到冲煮推荐结果的纯领域逻辑。

### 任务包

- M2.1 实现批次关联上下文解析：批次 -> 烘焙品类 -> 咖啡豆。
- M2.2 实现 OR 匹配逻辑。
- M2.3 实现多方案排序：
  - 匹配属性数量更少优先。
  - 再按 `priority`。
  - 再按分类和方案 `sort_order`。
- M2.4 实现养豆天数计算。
- M2.5 实现 day 0 到 day 14 线性拟合。
- M2.6 实现 TDS 水质修正，边界归入较低区间。
- M2.7 实现粉量 0.1g 步进后的总水量计算。
- M2.8 输出 UI DTO，例如 `BrewingRecommendation`。

### 测试覆盖

必须覆盖以下函数或等价函数：

- `match_brewing_plans(batch, state)`
  - 水洗命中水洗方案。
  - 日晒命中强甜感方案。
  - 深烘或印尼咖啡命中火山冲煮。
  - 无匹配时返回空列表。
- `sort_matched_plans(matches)`
  - 匹配属性更少的方案排在前面。
  - 匹配属性数量相同按 `priority` 排序。
- `calculate_age_days(roasted_at, now)`
  - 24 小时为 1 天。
  - 12 小时为 0.5 天。
- `fit_age_parameters(plan, age_days)`
  - day 0 返回 day 0 参数。
  - day 7 返回中间值。
  - day 14 返回 day 14 参数。
  - day 21 仍返回 day 14 参数。
- `apply_water_quality_adjustment(params, tds)`
  - TDS 60 归入 40-60。
  - TDS 80 归入 60-80。
  - TDS 150 归入 100-150。
  - TDS 151 命中 150+。
- `calculate_total_water(dose_g, ratio)`
  - 16g、1:16 得到 256g。
  - 15.5g、1:15 得到 232.5g。

### 客观验收

- M2 所有逻辑不依赖 UI、不依赖浏览器 API。
- `cargo test` 通过。
- 测试数据中至少包含 4 个 seed 冲煮方案。
- 推荐结果必须包含：
  - 方案名称。
  - 滤杯。
  - 磨豆机。
  - 拟合后的研磨刻度。
  - 拟合后的水温。
  - 粉量。
  - 总水量。
  - 注水段数。

### 不包含

- 不实现页面。
- 不实现远端保存。

## M3：本地状态与 Cloudflare API

### 目标

完成 AppState 的远端读写、本地缓存和 revision 冲突控制。

### 任务包

- M3.1 Worker API：`GET /api/state?store_id=...`。
- M3.2 Worker API：`PUT /api/state?store_id=...`。
- M3.3 KV key：`coffee_erp:store:{store_id}:state`。
- M3.4 首次 GET 不存在状态时，返回 seed state。
- M3.5 PUT 校验 revision，一致才写入 KV。
- M3.6 CORS 使用 `ALLOWED_ORIGINS`。
- M3.7 前端 API client 使用 `PUBLIC_API_BASE_URL`。
- M3.8 IndexedDB 缓存当前 AppState。
- M3.9 localStorage 保存 `store_id` 和 UI 偏好。
- M3.10 revision 冲突时阻止保存，并提示用户刷新远端最新数据；MVP 不做 diff。

### 测试覆盖

必须覆盖以下函数或等价函数：

- Worker:
  - `stateKey(store_id)`
  - `handleGetState(request, env)`
  - `handlePutState(request, env)`
  - `isAllowedOrigin(origin, allowed_origins)`
- Frontend storage:
  - `load_cached_state()`
  - `save_cached_state(state)`
  - `load_preferences()`
  - `save_preferences(preferences)`

### 客观验收

- GET 缺省 store 时返回 seed state。
- PUT 成功后 revision 增加 1。
- PUT revision 过期时返回冲突错误。
- CORS 拒绝不在 `ALLOWED_ORIGINS` 中的 Origin。
- 前端启动顺序为：
  - 先读 IndexedDB。
  - 再拉远端。
  - 远端成功后覆盖 IndexedDB。
- `cargo test` 通过。
- Worker 侧测试通过，使用项目选定的 JS/TS 测试命令。

### 不包含

- 不实现 UI 表单。
- 不实现部署。

## M4：资料维护界面

### 目标

实现资料维护页面，允许维护 seed data 生成后的业务数据。

### 任务包

- M4.1 参数目录标签页：豆种、烘焙度、处理法。
- M4.2 咖啡豆维护：名称、豆种、处理法、产地、备注。
- M4.3 烘焙方法维护。
- M4.4 烘焙品类维护：豆子、方法、烘焙度、产品线、`batch_code`。
- M4.5 `batch_code` 自动建议，保存前允许修改。
- M4.6 冲煮方案分类维护。
- M4.7 冲煮方案维护：匹配属性、注水段数、滤杯、磨豆机、比例、day 0/day 14、说明文字。
- M4.8 归档操作接入 pending undo。
- M4.9 表单校验错误展示。

### 测试覆盖

建议覆盖以下组件行为或等价交互：

- 新增咖啡豆后列表出现。
- 归档咖啡豆后默认列表隐藏。
- 新增烘焙品类时自动建议 `batch_code`。
- 冲煮方案可选择不同类型的匹配属性。
- 空名称保存失败。
- 保存后重新加载 AppState 数据仍存在。

### 客观验收

- 所有资料编辑都通过统一 AppState 保存流程。
- 表单错误不允许写入 KV。
- 归档动作出现 5 秒撤销。
- pending undo 期间所有写操作禁用。
- 移动端 390px 宽度下无明显横向溢出。

### 不包含

- 不实现入库。
- 不实现今日推荐详情。

## M5：入库与今日推荐

### 目标

完成用户日常核心流程：入库批次，查看养豆计时和冲煮推荐。

### 任务包

- M5.1 入库表单：烘焙品类、烘焙完成时间、批次数量、备注。
- M5.2 一次入库多个 100g 批次。
- M5.3 自动生成多个批次号。
- M5.4 批次列表和详情。
- M5.5 标记 UsedUp。
- M5.6 归档批次。
- M5.7 今日推荐页面展示 Active 批次。
- M5.8 推荐卡展示匹配方案和拟合参数。
- M5.9 粉量 0.1g 微调并实时更新总水量。
- M5.10 无匹配方案时允许查看全部方案。

### 测试覆盖

必须覆盖以下函数或等价交互：

- `create_batches(profile_id, roasted_at, count, state)`
  - count = 3 生成 3 个批次。
  - 批次号连续。
  - count = 0 校验失败。
- `visible_recommendation_batches(state)`
  - Active 显示。
  - UsedUp 不显示。
  - Archived 不显示。
- UI:
  - 入库 3 批后今日推荐出现 3 个 Active 批次。
  - 调整粉量从 16.0 到 16.1，总水量即时变化。
  - 标记 UsedUp 后批次从今日推荐消失。

### 客观验收

- 入库 3 批可生成连续编号。
- 今日推荐卡中必须展示：
  - 批次号。
  - 烘焙后天数。
  - 方案名。
  - 滤杯。
  - Ditting。
  - 研磨刻度。
  - 水温。
  - 粉量。
  - 总水量。
  - 注水段数。
- UsedUp / Archived 批次默认不参与今日推荐。
- 所有危险状态变更接入 5 秒撤销。

### 不包含

- 不实现按杯扣减。
- 不实现冲煮日志。

## M6：撤销、错误处理与移动端体验

### 目标

完成可用性和状态安全边界。

### 任务包

- M6.1 pending undo 状态管理。
- M6.2 撤销 toast 和 5 秒倒计时。
- M6.3 pending undo 期间禁用所有写操作。
- M6.4 保存失败恢复操作前状态。
- M6.5 revision 冲突提示强制刷新。
- M6.6 离线状态提示。
- M6.7 移动端布局、按钮、表单、列表空状态优化。

### 测试覆盖

建议覆盖以下行为：

- 危险操作后 5 秒内点击撤销，状态恢复且不调用远端保存。
- 5 秒倒计时结束后才调用远端保存。
- pending undo 期间第二个写操作按钮禁用。
- 保存失败恢复 `before_state`。
- revision 冲突提示刷新。

### 客观验收

- 所有危险操作都有撤销 toast。
- 5 秒内不能触发第二个写操作。
- 保存失败不会留下 UI 与本地缓存不一致的状态。
- 移动端 390px 和 430px 宽度下主要页面无横向滚动。

### 不包含

- 不实现多级撤销栈。
- 不实现本地快照。

## M7：部署文档与发布准备

### 目标

准备 Cloudflare 主站和 EdgeOne 前端镜像部署。

### 任务包

- M7.1 使用 `cloudflare-deploy` skill 编写 Cloudflare 部署说明。
- M7.2 使用 `edgeone-pages-deploy` skill 编写 EdgeOne Pages 部署说明。
- M7.3 说明环境变量：
  - `PUBLIC_API_BASE_URL`
  - `ALLOWED_ORIGINS`
  - `KV_NAMESPACE`
  - `STORE_ID_SEED`
- M7.4 说明构建产物和发布目录。
- M7.5 说明 CORS 验证步骤。
- M7.6 说明 smoke test 步骤。

### 测试覆盖

- Cloudflare smoke test：
  - GET state 成功。
  - PUT state 成功。
  - revision 冲突返回错误。
- EdgeOne smoke test：
  - 前端资源可访问。
  - 前端能请求 Cloudflare API。

### 客观验收

- Cloudflare 主站能读写 KV。
- EdgeOne 前端能访问 Cloudflare API。
- 不在文档中硬编码正式域名，全部通过环境变量说明。
- 文档足够按步骤复现部署。

### 不包含

- 不把 EdgeOne 作为 API 或 KV 主存储。

## 推荐顺序

1. M0 项目骨架。
2. M1 领域模型与 seed data。
3. M2 冲煮匹配与参数拟合。
4. M3 存储和 API。
5. M4 资料维护。
6. M5 入库和今日推荐。
7. M6 撤销和移动端体验。
8. M7 部署文档与发布准备。
