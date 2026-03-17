# AGENTS.md

本文件定义本仓库（`/Volumes/AppleSoft/media`）内所有 AI/人工协作者的统一执行规范。

## 1. 项目上下文（按当前历史）

- 目标项目：`lumenstream`（Rust 实现 Jellyfin 兼容服务）。
- 关联文档：
- 当前工作方式：功能实现 + 测试补齐 + 文档同步，保持可回放、可验证、可审计。

## 2. 代码风格与实现规范

### 2.1 通用规范
- 小步快改：每次变更聚焦单一功能点，避免“大杂烩”提交。
- 默认保持兼容：优先不破坏现有 API 行为与配置语义。
- 变更必须可验证：没有验证结果的功能视为未完成。

### 2.2 Rust 规范
- 统一使用 `cargo fmt --all`。
- 新增逻辑优先纯函数化，便于单元测试。
- 错误处理使用清晰上下文（`anyhow::Context` 等），避免静默失败。
- 新增配置项必须包含：默认值 + 配置样例 +（如适用）环境变量覆盖。

### 2.3 文档同步规范
以下场景必须更新文档（至少一处）：

## 3. 测试规范（强制）

### 3.1 功能与测试绑定
- 每个新增功能必须同时新增或更新测试。
- 每个修复缺陷必须有可复现测试（回归测试）。
- 严禁“只改功能不补测试”。

### 3.2 最低验证清单
提交前至少通过：
- `cargo fmt --all`
- `cargo test --workspace`
- `bash -n ../scripts/contract_test_api.sh ../scripts/replay_client_callchains.sh`

如改动涉及脚本行为、协议契约或调用链，需额外执行对应脚本验证。

### 3.3 前端测试
完成验证前，自行在 `web/` 目录下执行：
- `bun run dev`
随后使用 chrome-devtools mcp 对实际效果进行测试。需要从美观度，交互体验，功能完整性，视觉可读性等维度进行评估。

## 4. 提交规范（强制）

### 4.1 原子提交要求
- **每完成一个功能新增 + 对应测试通过后，立即 commit 一次。**
- 一个 commit 必须自洽：代码、测试、必要文档在同一提交中闭环。

### 4.2 建议提交信息格式
- `feat(scope): ...`
- `fix(scope): ...`
- `test(scope): ...`
- `docs(scope): ...`

示例：
- `feat(tmdb): add throttle/cache/retry with failure persistence`
- `test(api): cover metrics snapshot and scan payload parsing`

## 5. Pre-commit 规范（必须使用 Prek）

### 5.1 强制要求
- 所有提交前必须通过 **Prek pre-commit**。
- 未通过 Prek pre-commit，禁止提交。

### 5.2 执行方式（仓库规范）
- 每个 Git 子仓库以其根目录 `prek.toml` 为 pre-commit 规则源。
- 当前主项目使用 `lumenstream/prek.toml`。
- 在 `lumenstream` 仓库中统一执行：`prek run --all-files`。
- 若在工作区根目录执行，可使用：`prek -C lumenstream -c prek.toml run --all-files`。
- pre-commit 检查至少应覆盖：
  - 格式化检查（Rust fmt）
  - 测试执行（workspace tests）
  - 脚本语法检查

### 5.3 与 commit 的关系
- 顺序固定：**实现功能 → 新增/更新测试 → 运行 Prek pre-commit → commit**。
- 任一步失败，先修复再继续，不得跳过。

## 6. 禁止事项

- 禁止跳过测试直接提交。
- 禁止绕过 Prek pre-commit 提交。
- 禁止提交与当前功能无关的大量噪音改动。
- 禁止修改功能后不更新对应文档与测试。

---
如无特殊说明，本规范对本目录下所有子项目生效。
