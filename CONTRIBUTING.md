# Contributing

感谢你为 `lumenstream` 做贡献。

## Development model

- 本仓库当前作为 **CE upstream** 主线开发仓库。
- 商业版会作为 **downstream** 吸收 CE 的稳定改动。
- 因此，所有提交都必须保持：
  - 可验证
  - 可审计
  - 不破坏 CE 核心功能

## Required before submitting

1. 阅读并接受 `/Volumes/AppleSoft/media/lumenstream/CLA.md`
2. 遵守 `/Volumes/AppleSoft/media/lumenstream/AGENTS.md`
3. 本地至少通过：
   - `cargo fmt --all`
   - `cargo test --workspace`
   - `bash -n ../scripts/contract_test_api.sh ../scripts/replay_client_callchains.sh`
   - `prek run --all-files`

## Contribution boundaries

欢迎提交：

- Jellyfin / Emby 兼容修复
- 媒体库、扫描、刮削、搜索、播放核心能力
- Agent 与推流域名等 CE 保留能力
- 文档、测试、开发体验改进

商业版专属能力当前包括：

- 账单 / 钱包 / 套餐 / 订阅
- 高级用户流控
- 邀请奖励
- 审计 CSV 导出

涉及这些边界的改动，请同步更新：

- `/Volumes/AppleSoft/media/lumenstream/docs/editions.md`
- `/Volumes/AppleSoft/media/lumenstream/docs/upstream-downstream.md`

## Pull requests

- PR 应尽量保持单一主题
- 代码、测试、文档应同一提交闭环
- 提交 PR 即表示你同意 `/Volumes/AppleSoft/media/lumenstream/CLA.md`
