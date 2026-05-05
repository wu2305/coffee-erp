# Coffee ERP 技术实施方案

## 1. 产品定位

Coffee ERP 是一个移动端优先的咖啡豆入库、养豆计时与冲煮方案推荐工具。

当前业务形态：

- MVP 面向单个门店单独使用。
- 第一版只做基本入库、养豆计时和冲煮方案拟合。
- 烘焙机器单次容量为 100g，因此 100g 是天然批次单位。
- 店内人员彼此可信，MVP 不做账号、权限和角色管理。

系统不依赖传统 VPS、云数据库或常驻服务器。Cloudflare Pages 是主站，负责前端、API 和 KV 主数据；EdgeOne Pages 只部署前端静态资源镜像，用于优化中国大陆访问速度。

## 2. 技术路线

### 2.1 应用形态

- 类型：移动端优先的网页应用
- 部署：Cloudflare Pages 为主，EdgeOne Pages 只部署前端静态资源镜像
- 后端：无传统服务器，只有 serverless API
- 主存储：KV
- 本地缓存：IndexedDB 缓存最近一次完整数据
- 本地偏好：localStorage 只保存当前 store ID 和 UI 偏好
- 离线能力：后续通过 PWA 增强
- 主要使用场景：手机浏览器访问，或添加到主屏幕后作为类 App 使用

### 2.2 技术栈

推荐使用：

- Rust
- Dioxus Web
- WebAssembly
- serde / serde_json
- uuid 或同类 ID 生成方案
- chrono 或 time 处理日期时间
- Cloudflare Workers KV
- Cloudflare Pages Functions / Workers API
- EdgeOne Pages 静态资源部署
- IndexedDB

MVP 不使用关系型数据库。Cloudflare Workers KV 适合当前场景，原因是：

- 店长维护数据，写入频率低。
- 店员主要查看数据，读取频率高。
- 不存在多人高频协同编辑。
- 店内人员可信，不需要复杂权限系统。
- 数据规模小，可以整体保存为 JSON 文档。

需要接受的限制：

- KV 是最终一致性存储，保存后不同访问节点可能短时间内看到旧数据。
- 不适合多人同时编辑同一份数据。
- 不适合复杂查询和大量流水统计。
- 保存时以整个文档为单位写入。

如果未来出现高频多人编辑、严格库存流水、审计日志、复杂统计，再升级到 D1 或其他关系型数据库。

### 2.3 数据持久化策略

MVP 使用一个单店级 JSON 文档作为主数据，主数据保存在 Cloudflare Workers KV。

KV key 示例：

```text
coffee_erp:store:{store_id}:state
```

前端启动时：

1. 从 IndexedDB 读取最近缓存，先展示可用数据。
2. 调用 API 从 KV 拉取最新数据。
3. 如果远端数据更新，则替换 IndexedDB 缓存。

保存时：

1. 前端带上当前 `revision` 提交完整 AppState。
2. API 检查远端 `revision` 是否一致。
3. 一致则写入 KV，并将 `revision` 加 1。
4. 不一致则拒绝保存，提示用户刷新后再操作。

这样可以避免用户长时间打开旧页面后覆盖较新的数据。

冲突处理：

- 如果 A 保存后 B 再保存，B 的 `revision` 会过期。
- API 拒绝 B 的保存请求。
- 前端提示 B 必须刷新远端最新数据。
- MVP 不做差异对比，也不保留 B 的本地修改。

示例 API：

```text
GET /api/state?store_id=xxx
PUT /api/state?store_id=xxx
```

MVP 不实现登录认证。为了降低被随机猜中的概率，`store_id` 使用较长随机 ID。

### 2.4 部署策略

Cloudflare 是主运行环境：

- Cloudflare Pages 托管前端静态资源。
- Cloudflare Pages Functions / Workers 提供 API。
- Cloudflare Workers KV 保存主数据。
- 前端默认请求 Cloudflare API。

EdgeOne Pages 只作为中国大陆访问优化的前端静态资源镜像：

- EdgeOne 不承载业务 API。
- EdgeOne 不写入 KV。
- EdgeOne 页面中的 API base URL 通过环境变量指向 Cloudflare API。
- Cloudflare API 通过环境变量配置 CORS allowlist，允许 EdgeOne 前端访问。

这样可以保证：

- 主数据只有一份，避免 Cloudflare KV 与 EdgeOne KV 双写不一致。
- EdgeOne 提供更快的前端资源加载。
- 后端逻辑集中在 Cloudflare，后续升级 D1 也更直接。

部署环境变量：

```text
PUBLIC_API_BASE_URL
ALLOWED_ORIGINS
KV_NAMESPACE
STORE_ID_SEED
```

说明：

- `PUBLIC_API_BASE_URL` 由前端构建使用，EdgeOne 镜像和 Cloudflare 主站可以配置不同值。
- `ALLOWED_ORIGINS` 由 Cloudflare API 使用，用于 CORS 校验。
- `KV_NAMESPACE` 由 Cloudflare API 绑定 Workers KV。
- `STORE_ID_SEED` 可用于初始化默认 store，实际 store ID 仍使用较长随机 ID。

### 2.5 本地存储分工

浏览器端同时使用 IndexedDB 和 localStorage，但职责不同。

IndexedDB 保存：

- 最近一次完整 AppState。
- 危险操作提交前的临时撤销状态。
- 离线编辑草稿，后续如需要再启用。

localStorage 保存：

- 当前 `store_id`。
- UI 偏好，例如默认页面、列表筛选项。
- 最后一次同步时间。

约束：

- 业务数据不得以 localStorage 作为主要缓存。
- UI 层不直接访问 IndexedDB，由 `storage` 模块统一封装。
- KV 仍然是多人共享的主数据源，IndexedDB 只是本机缓存。

## 3. 核心业务概念

### 3.1 门店

门店表示当前 MVP 的单店业务空间。

```rust
struct Store {
    id: String,
    name: String,
    water_tds: Option<f32>,
}
```

MVP 只维护一个门店，不提供多门店切换、批次流转和门店权限。

`water_tds` 表示门店当前冲煮用水的固定 TDS 设置，用于水质修正。MVP 可为空；为空时不应用水质修正。

该字段虽然当前只服务单店，但需要保留为正式模型的一部分，后续给其他用户使用时可按门店维护不同水质。

### 3.2 参数目录

品种、烘焙度、处理法属于业务枚举，但不应写死为 Rust `enum`。

MVP 将它们作为单店级参数目录保存到 AppState 中。系统首次初始化时预置商家提供的默认值，后续允许在界面中新增、重命名、排序和归档。

首次创建 store 时，系统写入 seed data。之后这些数据都视为普通业务数据，可在页面中继续维护，不再跟随代码自动覆盖。

```rust
struct CoffeeParameters {
    bean_varieties: Vec<CatalogOption>,
    roast_levels: Vec<RoastLevelOption>,
    processing_methods: Vec<CatalogOption>,
}

struct CatalogOption {
    id: String,
    label: String,
    sort_order: u32,
    archived: bool,
}

struct RoastLevelOption {
    id: String,
    label: String,
    agtron_range: String,
    agtron_min: Option<f32>,
    agtron_max: Option<f32>,
    sort_order: u32,
    archived: bool,
}
```

初始化参数：

```json
{
  "bean_varieties": [
    "瑰夏/希爪种",
    "埃塞原生 (74系)",
    "波旁",
    "铁皮卡/卡杜拉/",
    "象豆种",
    "印尼咖啡"
  ],
  "roast_levels": [
    { "label": "极浅", "agtron_range": "95+" },
    { "label": "浅", "agtron_range": "90-95" },
    { "label": "浅中", "agtron_range": "80-90" },
    { "label": "中", "agtron_range": "70-80" },
    { "label": "中深", "agtron_range": "60-70" },
    { "label": "深", "agtron_range": "50-60" }
  ],
  "processing_methods": [
    "日晒",
    "水洗",
    "蜜处理",
    "轻厌氧",
    "强厌氧",
    "增味"
  ]
}
```

设计原则：

- 代码中不使用 `enum BeanVariety`、`enum RoastLevel`、`enum ProcessingMethod`。
- 业务数据通过 `*_id` 引用目录项。
- 已被引用的目录项不直接删除，只允许归档。
- 归档目录项仍可在历史数据中展示。
- 新增目录项不需要发版。
- `agtron_range` 保留原始展示文本，`agtron_min` 和 `agtron_max` 只作为可选结构化字段。

### 3.3 咖啡豆

咖啡豆表示一种原料。

```rust
struct CoffeeBean {
    id: String,
    name: String,
    variety_id: Option<String>,
    processing_method_id: Option<String>,
    origin: Option<String>,
    notes: Option<String>,
    archived: bool,
}
```

说明：

- `name` 为必填。
- `variety_id` 引用参数目录中的豆种。
- `processing_method_id` 引用参数目录中的处理法。
- `origin` 可记录产地。
- `archived` 用于归档，不直接删除历史数据依赖。

### 3.4 烘焙方法

烘焙方法表示一种烘焙策略或烘焙曲线。

```rust
struct RoastMethod {
    id: String,
    name: String,
    notes: Option<String>,
    archived: bool,
}
```

示例：

- 快速升温曲线
- 延长发展曲线
- 标准手冲曲线
- 意式拼配曲线
- 某个固定烘焙曲线名

### 3.5 产品线

产品线用于区分手冲和意式。

```rust
enum ProductLine {
    PourOver,
    Espresso,
}
```

业务规则：

- 手冲品类需要维护冲煮参数。
- 意式品类不维护手冲注水参数。
- 意式品类需要维护建议养豆天数，便于门店判断是否适合开始消耗。

### 3.6 烘焙品类

烘焙品类是咖啡豆、烘焙方法、烘焙度和产品线的组合。

```rust
struct RoastProfile {
    id: String,
    bean_id: String,
    method_id: String,
    roast_level_id: Option<String>,
    product_line: ProductLine,
    display_name: String,
    batch_code: String,
    recommended_rest_days: Option<u32>,
    espresso_note: Option<String>,
    archived: bool,
}
```

业务规则：

- 同一种咖啡豆使用不同烘焙方法，视为不同烘焙品类。
- 同一种咖啡豆使用不同烘焙度，视为不同烘焙品类。
- 同一种咖啡豆用于手冲和意式，也视为不同烘焙品类。
- `ProductLine::PourOver` 通过冲煮方案目录匹配推荐方案。
- `ProductLine::Espresso` 不参与手冲冲煮方案匹配。
- `recommended_rest_days` 对手冲和意式都可使用，但意式更重要。
- `espresso_note` 只用于意式备注，不维护结构化压力、粉量、出液量等参数。
- `batch_code` 用于自动生成批次编号。

### 3.7 冲煮方案目录

冲煮方案目录保存商家定义的推荐冲煮方案和匹配规则。

这些方案是初始化数据，但不写死在代码中。后续允许新增、调整、归档。

初始化时统一规范为“分类包含多个方案”的结构。即使某个分类当前只有一个方案，也保存为 `plans: Vec<BrewingPlan>`，避免代码处理两种 JSON 形状。

```rust
struct BrewingPlanCategory {
    id: String,
    name: String,
    sort_order: u32,
    plans: Vec<BrewingPlan>,
    archived: bool,
}

struct BrewingPlan {
    id: String,
    name: String,
    matching_attributes: Vec<BrewingMatchAttribute>,
    parameters: BrewingPlanParameters,
    age_fitting: BrewingAgeFitting,
    instructions: Option<String>,
    priority: u32,
    archived: bool,
}

struct BrewingMatchAttribute {
    kind: BrewingMatchKind,
    option_id: String,
}

enum BrewingMatchKind {
    BeanVariety,
    ProcessingMethod,
    RoastLevel,
}

struct BrewingPlanParameters {
    pour_stages: u8,
    dripper: String,
    grinder_profile_id: Option<String>,
    ratio: BrewRatio,
    default_dose_g: f32,
}

struct GrinderProfile {
    id: String,
    name: String,
    notes: Option<String>,
    archived: bool,
}

struct BrewingAgeFitting {
    day0: BrewingAgeEndpoint,
    day14: BrewingAgeEndpoint,
}

struct BrewingAgeEndpoint {
    grind_size: f32,
    water_temp_c: f32,
}

struct WaterQualityAdjustment {
    tds_min: Option<f32>,
    tds_max: Option<f32>,
    temp_mod_c: f32,
    grind_mod: f32,
    label: String,
}

struct BrewRatio {
    coffee: f32,
    water: f32,
}
```

初始化方案：

- `强层次，强风味`
  - `锥形滤杯一刀流`
    - 匹配：水洗、轻厌氧
    - day 0：Ditting 5.5、96 C
    - day 14：Ditting 6.0、94 C
    - 参数：2 段、RF、1:16、默认粉量 16g
  - `标准三段式`
    - 匹配：水洗
    - day 0：Ditting 7.1、93 C
    - day 14：Ditting 7.5、91 C
    - 参数：3 段、山文、1:16、默认粉量 16g
- `强甜感`
  - `蛋糕滤杯一刀流`
    - 匹配：日晒、轻厌氧、强厌氧
    - day 0：Ditting 6.5、93 C
    - day 14：Ditting 7.0、91 C
    - 参数：2 段、马赫、1:15、默认粉量 16g
- `深烘`
  - `火山冲煮`
    - 匹配：深烘、印尼咖啡
    - day 0：Ditting 8.5、85 C
    - day 14：Ditting 8.8、85 C
    - 参数：1 段、马赫、1:13、默认粉量 16g

匹配规则：

- `matching_attributes` 使用 OR 逻辑，命中任意一个属性即匹配。
- 匹配属性必须引用参数目录中的目录项 ID，不使用字符串硬匹配。
- 匹配属性可以来自不同目录，例如处理法、豆种、烘焙度。
- `轻厌氧` 和 `强厌氧` 是处理法目录中的标准值。
- 用户查看某个批次时，系统根据该批次关联的咖啡豆和烘焙品类匹配可用冲煮方案。
- 多个冲煮方案同时命中时，全部展示。
- 展示顺序优先显示匹配属性数量更少的方案，即规则更具体的方案优先。
- 如果匹配属性数量相同，再按 `priority` 和 `sort_order` 排序。
- 粉量默认 16g，允许用户按 0.1g 步进微调。
- 总水量根据粉量和比例实时计算。
- MVP seed data 只初始化一个磨豆机：Ditting。
- 数据模型保留 `GrinderProfile`，后续可以为其他用户增加更多磨豆机。
- 第一版的研磨刻度仍直接保存在冲煮方案的 day 0 / day 14 参数中。
- `instructions` 是冲煮方案说明文字，MVP 字段保留但初始值为空。

水质修正初始化规则：

- TDS 40-60：不调整。
- TDS 60-80：水温 -1 C。
- TDS 80-100：研磨刻度 +0.1。
- TDS 100-150：水温 -1 C，研磨刻度 +0.1。
- TDS 150+：水温 -2 C，研磨刻度 +0.2。

边界规则：

- TDS 正好等于区间边界时，归入较低区间。
- 示例：TDS = 60 归入 40-60；TDS = 80 归入 60-80；TDS = 100 归入 80-100；TDS = 150 归入 100-150。

拟合优先级：

- 优先调节水温。
- 再调整研磨度。

### 3.8 批次

批次表示烘焙机一次 100g 产出的自然生产单位。

```rust
struct RoastBatch {
    id: String,
    profile_id: String,
    roasted_at: String,
    batch_no: String,
    status: BatchStatus,
    notes: Option<String>,
}

enum BatchStatus {
    Active,
    UsedUp,
    Archived,
}
```

业务规则：

- 一个批次固定为 100g。
- MVP 中用户手工入库时生成批次。
- 批次的养豆时间始终按 `roasted_at` 计算，不按转移时间或到店时间计算。
- Active 批次参与今日冲煮推荐。
- UsedUp 批次保留历史记录，但默认不参与今日冲煮推荐。
- Archived 批次默认不展示在日常列表中。

#### 3.8.1 批次编号

批次编号自动生成，在用户入库时生成正式编号。

原因：

- 入库时间才代表实际烘焙完成时间。
- 入库时才知道实际批次数量。

正式批次编号在入库并生成批次时创建。

编号格式：

```text
{roasted_date_yyyyMMdd}-{batch_code}-{daily_sequence}
```

示例：

```text
20260502-YJPO-001
20260502-YJPO-002
20260502-ESP-003
```

规则：

- 日期使用实际烘焙日期。
- `batch_code` 来自 `RoastProfile.batch_code`。
- `daily_sequence` 是当天全部批次的全局递增序号。
- 批次编号默认不允许修改，异常情况写入备注。
- `batch_code` 默认由系统根据烘焙品类名称自动建议，保存前允许用户修改。

## 4. AppState

MVP 使用一个单店级 AppState 文档保存所有数据。

```rust
struct AppState {
    schema_version: u32,
    revision: u64,
    store: Store,
    coffee_parameters: CoffeeParameters,
    grinder_profiles: Vec<GrinderProfile>,
    water_quality_adjustments: Vec<WaterQualityAdjustment>,
    brewing_plan_categories: Vec<BrewingPlanCategory>,
    beans: Vec<CoffeeBean>,
    roast_methods: Vec<RoastMethod>,
    roast_profiles: Vec<RoastProfile>,
    batches: Vec<RoastBatch>,
    updated_at: String,
}
```

说明：

- `schema_version` 用于后续数据迁移。
- `revision` 用于避免旧页面覆盖新数据。
- 所有业务数据整体存入 KV。
- IndexedDB 保存本机缓存副本。
- localStorage 只保存当前 store ID 和 UI 偏好。

## 5. 养豆计时与冲煮方案拟合

### 5.1 天数计算

每个批次独立计算烘焙后天数：

```text
age_days = 当前日期时间 - roasted_at
```

MVP 建议使用小时差折算：

```text
age_days = elapsed_hours / 24
```

这样同一天内也可以得到更精确的养豆时间。

### 5.2 方案匹配

系统根据批次关联的咖啡豆和烘焙品类匹配冲煮方案。

可参与匹配的属性：

- 咖啡豆的豆种。
- 咖啡豆的处理法。
- 烘焙品类的烘焙度。

匹配逻辑：

- 单个方案内的 `matching_attributes` 使用 OR 逻辑。
- 命中任意一个属性即认为方案可用。
- 如果没有命中任何方案，展示“无匹配方案”，允许用户查看全部方案并手动选择。
- 如果命中多个方案，MVP 暂时全部展示，后续可通过 `priority` 做默认排序。

### 5.3 参数拟合

冲煮方案参数来自 `BrewingPlanParameters` 和 `BrewingAgeFitting`。

拟合顺序：

1. 根据批次匹配冲煮方案。
2. 根据养豆天数在 day 0 与 day 14 之间拟合水温和研磨度。
3. 根据门店水质 TDS 叠加水质修正。
4. 根据粉量和比例计算总水量。

#### 5.3.1 养豆天数拟合

拟合范围限定为第 0 天到第 14 天：

```text
age_ratio = clamp(age_days / 14, 0, 1)
```

水温和研磨度使用线性拟合：

```text
age_temp_c = day0.water_temp_c + (day14.water_temp_c - day0.water_temp_c) * age_ratio
age_grind_size = day0.grind_size + (day14.grind_size - day0.grind_size) * age_ratio
```

超过 14 天时，继续使用 day 14 参数，不继续外推。

示例：

```text
蛋糕滤杯一刀流
day 0:  grind 6.5, temp 93
day 14: grind 7.0, temp 91
day 7:  grind 6.75, temp 92
```

#### 5.3.2 水质修正

如果门店维护了 `water_tds`，系统根据 TDS 区间叠加修正：

```text
final_temp_c = age_temp_c + temp_mod_c
final_grind_size = age_grind_size + grind_mod
```

如果没有维护 `water_tds`，则：

```text
final_temp_c = age_temp_c
final_grind_size = age_grind_size
```

同一 TDS 值只命中一个区间。

#### 5.3.3 粉量与总水量

默认粉量：

```text
dose_g = 16.0
```

用户可以按 0.1g 步进调整粉量。

总水量根据比例计算：

```text
total_water_g = dose_g * ratio.water / ratio.coffee
```

示例：

```text
ratio = 1:16
dose_g = 16.0
total_water_g = 256.0
```

滤杯、磨豆机、比例和注水段数直接来自冲煮方案。
水温和研磨刻度来自拟合结果。

### 5.4 分段说明

MVP 只展示方案中的注水段数，不维护每段注水量和每段注水时间。

约定：

- `pour_stages = 1` 表示一段注水，例如火山冲煮一口气注水。
- `pour_stages = 2` 表示两段注水。
- `pour_stages = 3` 表示三段注水。
- 具体操作说明使用 `instructions` 描述。MVP 预留字段，但初始化数据可为空。

### 5.5 输出展示

展示给用户的拟合结果建议四舍五入：

- 水温：保留 1 位小数或整数
- 粉量：保留 1 位小数
- 总水量：保留 1 位小数或整数
- 研磨刻度：保留 1 位小数

内部计算保留浮点值，展示层再格式化。

## 6. 页面结构

MVP 页面以移动端底部导航组织。

### 6.1 今日推荐

展示当前 Active 批次和匹配到的冲煮方案。

展示字段：

- 咖啡豆名称
- 烘焙方法
- 批次编号
- 烘焙后天数
- 匹配到的冲煮方案名称
- 滤杯
- 磨豆机
- 研磨刻度
- 粉量
- 总水量
- 水温
- 比例
- 注水段数
- 冲煮说明

支持操作：

- 调整粉量，步进 0.1g。
- 在多个匹配方案之间切换。
- 查看无匹配方案时的全部方案。

排序建议：

1. Active 批次优先。
2. 烘焙时间较早的批次优先。

### 6.2 入库

功能：

- 新增入库批次。
- 查看批次列表。
- 查看批次详情。
- 归档批次。

新增入库时需要选择：

- 烘焙品类。
- 实际烘焙完成时间。
- 批次数量。
- 备注。

系统按批次数量生成多个 100g 批次，并自动生成批次编号。

### 6.3 资料

功能：

- 维护豆种、烘焙度、处理法参数目录。
- 维护咖啡豆。
- 维护烘焙方法。
- 维护烘焙品类。
- 维护冲煮方案目录。

MVP 可以先将参数目录、咖啡豆、烘焙方法、烘焙品类放在同一页面的不同标签中。

### 6.4 设置

功能：

- 拉取最新远端数据。
- 查看数据版本和 revision。
- 清空 IndexedDB 本地缓存。
- 重置本地 store ID。

清空远端 KV 数据不作为 MVP 功能。

## 7. 误操作恢复

MVP 支持单个危险操作的 5 秒撤销窗口。

### 7.1 危险操作范围

以下操作属于危险操作：

- 归档批次。
- 归档咖啡豆。
- 归档烘焙方法。
- 归档烘焙品类。
- 归档参数目录项。
- 归档冲煮方案。

### 7.2 撤销规则

规则：

1. 危险操作执行后，前端进入 pending undo 状态。
2. pending undo 持续 5 秒。
3. pending undo 期间，所有写操作禁用。
4. 用户点击撤销时，恢复操作前状态，不写入 KV。
5. 用户等待倒计时结束时，提交操作后状态到 KV。
6. 提交成功后更新 IndexedDB 缓存，并解除写操作禁用。
7. 提交失败时恢复操作前状态，并提示保存失败。

普通浏览、筛选和页面切换不受 pending undo 影响。

MVP 同一时间只允许存在一个 pending undo，不支持多个撤销栈。

示例结构：

```rust
struct PendingUndoAction {
    id: String,
    label: String,
    before_state: AppState,
    after_state: AppState,
    expires_at: String,
}
```

说明：

- MVP 数据量小，允许保存完整 `before_state` 和 `after_state`。
- 后续如数据量增大，再改为 patch 或操作日志。

### 7.3 备份策略

MVP 不提供 JSON 导入或导出，因为期初数据量不大，错误数据可以直接在界面维护。

MVP 不提供最近 N 份本地快照。IndexedDB 只保存当前缓存和 pending undo 临时状态。

不规划 JSON 导入功能，避免误覆盖远端 KV 主数据。后续如需备份，只考虑只读导出，不做导入恢复。

## 8. MVP 范围

第一版实现：

- 移动端优先网页应用。
- KV 主存储。
- IndexedDB 本地缓存。
- localStorage 本地偏好。
- revision 保存冲突检查。
- 参数目录管理。
- 咖啡豆管理。
- 烘焙方法管理。
- 烘焙品类管理。
- 冲煮方案目录管理。
- 手工入库生成 100g 批次。
- 批次养豆计时。
- 冲煮方案匹配。
- 粉量 0.1g 步进微调。
- 根据粉量和比例计算总水量。
- 危险操作 5 秒撤销。
- pending undo 期间禁用所有写操作。

第一版暂不实现：

- 用户账号。
- 权限管理。
- 多人协同编辑。
- D1 / 关系型数据库。
- 每杯咖啡的克重扣减。
- 复杂库存流水。
- 冲煮日志。
- 烘焙计划。
- 多门店。
- 批次流转。
- 意式库存消耗。
- 统计报表。
- 推送提醒。
- 图片附件。
- JSON 导出。
- JSON 导入。
- 多语言。

## 9. 数据校验规则

### 9.1 门店

- 名称不能为空。
- 如果填写 `water_tds`，必须大于 0。

### 9.2 咖啡豆

- 名称不能为空。
- 如果填写豆种，必须引用存在且未归档的豆种目录项。
- 如果填写处理法，必须引用存在且未归档的处理法目录项。
- 已被烘焙品类引用的咖啡豆不直接删除，只允许归档。

### 9.3 烘焙方法

- 名称不能为空。
- 已被烘焙品类引用的烘焙方法不直接删除，只允许归档。

### 9.4 烘焙品类

- 必须关联一个咖啡豆。
- 必须关联一个烘焙方法。
- 如果填写烘焙度，必须引用存在且未归档的烘焙度目录项。
- 必须指定产品线。
- `batch_code` 不能为空。

### 9.5 参数目录

- 目录项名称不能为空。
- 同一目录中不允许存在两个未归档且名称完全相同的目录项。
- 已被引用的目录项不直接删除，只允许归档。
- 烘焙度的 `agtron_range` 可以为空，但如果填写 `agtron_min` 和 `agtron_max`，必须满足 `agtron_min <= agtron_max`。

### 9.6 冲煮方案

- 名称不能为空。
- 滤杯不能为空。
- 注水段数必须大于 0。
- 比例中的咖啡量和水量必须大于 0。
- 默认粉量必须大于 0。
- day 0 和 day 14 的水温必须大于 0。
- day 0 和 day 14 的研磨刻度必须大于 0。
- 匹配属性必须引用存在的参数目录项。
- `priority` 必须大于等于 0。

### 9.7 批次

- 必须关联一个烘焙品类。
- 烘焙完成时间不能为空。
- 一个批次固定为 100g。
- Active 批次参与今日推荐。
- UsedUp 批次不参与今日推荐。
- Archived 批次不参与日常列表。

## 10. 项目结构建议

建议后续将项目整理为：

```text
coffee-erp/
  Cargo.toml
  Dioxus.toml
  docs/
    implementation-plan.md
  src/
    main.rs
    app.rs
    domain/
      mod.rs
      models.rs
      brewing_match.rs
      batch_number.rs
      validation.rs
    storage/
      mod.rs
      api_client.rs
      indexed_db_cache.rs
      preferences.rs
      undo.rs
    ui/
      mod.rs
      routes.rs
      today.rs
      inbound.rs
      batches.rs
      profiles.rs
      brewing_plans.rs
      settings.rs
  worker/
    src/
      index.ts
```

职责划分：

- `domain`：纯业务模型、冲煮方案匹配、批次编号、校验，不依赖 UI。
- `storage`：API 调用、IndexedDB 缓存、localStorage 偏好、撤销状态、schema 迁移。
- `ui`：Dioxus 页面和组件。
- `worker`：serverless API 和 KV 读写。

## 11. 后续演进方向

可在 MVP 稳定后考虑：

- 添加 PWA manifest 和 service worker。
- 添加 D1，支持更严格的库存流水和查询。
- 支持冲煮日志。
- 支持按杯或按克扣减库存。
- 支持到期提醒。
- 支持按养豆天数动态调整冲煮参数。
- 支持多磨豆机刻度，例如 Ditting 以外的其他磨豆机。
- 支持多门店。
- 支持烘焙计划。
- 支持标签打印或二维码。
- 支持更正式的权限和操作日志。
- 支持只读 JSON 导出备份。

## 12. 当前决策

- Cloudflare Pages 是主站，EdgeOne Pages 只部署前端静态资源镜像。
- 维护 Cloudflare Pages 和 EdgeOne Pages 两套前端部署文档。
- API base URL、CORS allowlist、KV 绑定和 store 初始化值通过环境变量配置。
- MVP 接受“知道 store_id 的人即可读写”的弱访问控制。
- PWA 是否进入第一版暂不作为阻塞项，基础 MVP 完成后再评估。
