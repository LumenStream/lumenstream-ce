# CE upstream / Commercial downstream

## Target model

- 本仓库：`lumenstream`，作为 **Community Edition upstream**
- 商业版仓库：私有 downstream
- 商业版定期吸收 CE 的稳定修复与功能改进

## Recommended flow

1. 在 CE upstream 完成核心功能、兼容修复、测试和文档
2. 商业版 downstream 定期从 CE upstream 拉取更新
3. 商业版仅在 downstream 中叠加商业专属模块

## Local bootstrap

可直接在当前工作区旁边切出两个本地仓库：

```bash
bash /Volumes/AppleSoft/media/lumenstream/scripts/cut_split_repositories.sh --force /Volumes/AppleSoft/media
```

完成后会得到：

- `/Volumes/AppleSoft/media/lumenstream-ce`
- `/Volumes/AppleSoft/media/lumenstream-commercial`

其中 commercial 会自动配置本地 `upstream` 指向 CE 仓库，便于后续执行：

```bash
git fetch upstream main
git merge --no-ff upstream/main
```

## Shared core

建议长期稳定保留在 CE upstream / shared core 的目录：

- `/Volumes/AppleSoft/media/lumenstream/crates/ls-agent`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-api`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-app`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-config`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-domain`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-infra`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-logging`
- `/Volumes/AppleSoft/media/lumenstream/crates/ls-scraper`
- `/Volumes/AppleSoft/media/lumenstream/web`

## Commercial overlay

建议未来在 downstream 中独立维护的能力：

- 账单 / 钱包 / 套餐 / 订阅
- 高级用户流控
- 邀请奖励
- 审计导出

当前这些能力已先以 capability gating 方式切出，尚未完全物理拆分到独立 crate。

## Rights management

为了确保 downstream 可以合法吸收 upstream 的贡献，必须要求所有外部贡献遵守：

- `/Volumes/AppleSoft/media/lumenstream/CLA.md`

没有 CLA 的外部贡献，不应直接进入未来商业版下游链路。
