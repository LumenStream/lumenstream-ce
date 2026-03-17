# Repository split plan

## Goal

将当前单仓逐步演进为：

- `lumenstream-ce`：公开 CE upstream
- `lumenstream-commercial`：私有 downstream

## Current strategy

当前阶段先保持单仓，原因：

- Jellyfin / Emby 兼容层仍在高频演进
- Core / commercial 边界刚稳定，需要继续观察
- 单仓更利于修复兼容问题与共享测试

## Split phases

### Phase 1: capability split

已完成：

- CE 默认关闭商业能力
- Agent / 推流域名 / LumenBackend 保留在 CE
- 前后端导航、接口、响应开始按 CE/EE 收口

### Phase 2: source split

进行中：

- 后端商业路由已收拢到 `crates/ls-api/src/api/router_commercial.rs`
- 前端商业 Admin API 已收拢到 `web/src/lib/api/admin-commercial.ts`
- 前端商业能力类型已收拢到 `web/src/lib/types/edition-commercial.ts`
- infra 版型判断开始收拢到 `crates/ls-infra/src/infra/helpers_edition.rs`

下一步建议：

- 将商业版专属页面与状态管理继续减少对 shared aggregator 的直接依赖
- 为未来独立 crate / package 做准备

### Phase 3: repo split

已开始落地：

- 可用 `scripts/cut_split_repositories.sh` 一次性生成：
  - `lumenstream-ce`
  - `lumenstream-commercial`
- 导出后的两个仓库都会写入：
  - `.lumenstream-repository-role.toml`
  - `README.generated.md`
  - `docs/generated-repository.md`
- commercial 仓库会自动配置本地 `upstream` 指向 CE 仓库

当前策略：

- CE 仓库保留 shared core 与 capability gating
- commercial 仓库在其上增加私有 overlay

本地切割示例：

```bash
bash scripts/cut_split_repositories.sh --force /Volumes/AppleSoft/media
```

## Helper scripts

可使用以下辅助脚本：

- `/Volumes/AppleSoft/media/lumenstream/scripts/export_ce_upstream.sh`
- `/Volumes/AppleSoft/media/lumenstream/scripts/init_commercial_downstream.sh`
- `/Volumes/AppleSoft/media/lumenstream/scripts/cut_split_repositories.sh`
- `/Volumes/AppleSoft/media/lumenstream/scripts/sync_from_ce_upstream.sh`
