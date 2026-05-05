# Coffee ERP 测试用例编写要求

## 基本原则

- 每个测试名称必须描述一个具体业务行为，例如 `fit_age_parameters_returns_midpoint_values_on_day7`。
- 测试结构保持 Arrange / Act / Assert：先准备数据，再调用目标函数，最后断言结果。
- 优先测试纯领域函数；UI、IndexedDB、KV API 在对应里程碑再补集成测试。
- 每个里程碑新增功能必须同时补测试，测试目标要能映射到 `docs/milestones.md` 的验收标准。

## 断言要求

- 对确定值使用 `assert_eq!(actual, expected)`，不要只用 `assert!(contains)` 验证局部结果。
- 对返回列表，如果完整顺序可确定，必须比较完整列表，避免只检查“包含某项”。
- 对校验错误，必须比较错误字段列表和数量，避免只检查“存在某个错误”。
- 对浮点拟合结果使用局部 `assert_approx_eq(actual, expected)` 辅助函数，并在辅助函数中固定容差。
- 对失败分支使用 `expect_err("...")`，对成功分支使用 `expect("...")` 或 `assert_eq!(result, Ok(...))`，错误说明必须能定位测试意图。

## 覆盖要求

- M1 领域模型测试必须覆盖 seed data 数量、枚举值、JSON roundtrip、批次编号、基础校验错误。
- M2 冲煮逻辑测试必须覆盖 OR 匹配、匹配排序、意式/非手冲不匹配、0/7/14/14+ 天拟合、TDS 边界、剂量 0.1g 归一、总水量计算、UI 推荐 DTO。
- 后续存储和 API 测试必须覆盖 revision 冲突、KV 空值初始化、非法请求、CORS 配置、本地缓存读写失败。

## 禁止事项

- 不允许用 `#[allow(dead_code)]` 或同类方式绕过未使用代码问题。
- 不允许用 `todo!()`、`unimplemented!()` 留占位通过编译。
- 不允许为了让测试通过而删除业务断言或放宽断言范围。
- 不允许把需要稳定校验的业务逻辑只放在人工检查里。
