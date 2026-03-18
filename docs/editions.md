# Editions

## Community Edition (CE)

默认版本，面向自托管与社区协作。

保留能力：

- Jellyfin / Emby 兼容核心 API
- 媒体库管理、扫描、刮削、搜索
- 用户登录、资料、会话、基础后台
- Agent 求片工作流（本地 TMDB 元数据优先匹配，再按 MoviePilot 精确搜索接口拼接完整 query 执行一次站点搜索）
- 播放域名、LumenBackend 推流与多端播放链路

## Commercial Edition (EE)

在 CE 之上开放以下扩展能力：

- 账单、钱包、套餐、充值、订阅
- 高级用户流控（有效期、并发限制、流量配额、Top 用量、重置）
- 邀请奖励（新用户赠送、邀请返利）
- 审计日志 CSV 导出

## Runtime switches

通过环境变量选择版本：

```bash
export LS_EDITION=ce
# or
export LS_EDITION=ee
```

可选能力覆盖：

```bash
export LS_EDITION_BILLING_ENABLED=false
export LS_EDITION_ADVANCED_TRAFFIC_CONTROLS_ENABLED=false
export LS_EDITION_INVITE_REWARDS_ENABLED=false
export LS_EDITION_AUDIT_LOG_EXPORT_ENABLED=false
export LS_EDITION_REQUEST_AGENT_ENABLED=true
export LS_EDITION_PLAYBACK_ROUTING_ENABLED=true
```
