# AnyRouter Claude Code 活性守护器计划文档

## 1. 背景

当前 Claude Code 通过 AnyRouter 一类中转网关接入模型服务。该类网关存在“活跃用户/活跃窗口”机制：用户不活跃时，请求可能长期返回 `429`；只有持续参与活跃队列，才更容易进入可用状态。

因此，本项目不是传统意义上的 API 健康检查器，也不是遇到 `429` 就退避的限流客户端，而是一个本地跨端桌面 App，用于在指定时间窗口内持续发起轻量 Claude Code 请求，帮助保持 AnyRouter 活跃状态，并提供可视化监控与请求历史。

## 2. 产品目标

构建一个基于 Tauri 的跨端桌面 App：

- 在每天 `05:00-24:00` 之间持续随机探测 AnyRouter 可用性。
- 每 `60-120s` 随机发起一次轻量 Claude Code 调用。
- 遇到 `429 / 503 / 524 / ECONNRESET / overloaded / timeout` 不退避，继续维持随机探测。
- 提供好看的状态界面、请求历史、24 小时活性监控。
- App 自身日志不能出现高频刷盘、无限增长、异常重试写盘风暴。
- 密钥安全存储，日志严格脱敏。

## 3. 非目标

第一版不做以下事情：

- 不保证一定能抢到 AnyRouter 活跃窗口，只提升活跃探测概率。
- 不绕过 AnyRouter 的认证、计费或服务规则。
- 不做每日请求数上限，因为该场景下请求频率本身就是产品核心。

## 4. 技术选型

- 桌面框架：Tauri
- 后端：Rust
- 前端：React + TypeScript + Vite
- UI 组件：shadcn/ui
- 无障碍基础：Radix UI
- 样式：Tailwind CSS v4
- 图标：lucide-react
- 表格：TanStack Table
- 图表：Recharts + 自定义 24h heatmap
- 状态管理：Zustand
- 本地数据库：SQLite
- 密钥存储：系统 Keychain / Credential Manager / libsecret
- Claude 调用：Rust `tokio::process::Command`
- 后台能力：Tauri tray + autostart

### 4.1 UI 选型说明

前端 UI 使用 `shadcn/ui + Radix UI + Tailwind CSS v4`。

选择原因：

- 当前 React 仪表盘、SaaS、桌面壳应用里热度高，生态活跃。
- 组件源码进入项目内，便于做精细视觉控制，不被重型主题系统限制。
- Radix UI 提供弹窗、菜单、Tooltip、Select 等无障碍交互基础。
- Tailwind CSS v4 适合快速搭建精致、统一、可维护的桌面端界面。
- shadcn/ui 组件和 Tauri + React 组合已有较多实践，适合本项目这种状态看板和设置型工具。

设计基调：

- 不是营销页风格，而是精致的桌面工具。
- 信息密度高，但层次清楚。
- 首页优先展示连接状态、最近成功时间、下一次 probe 倒计时、24h 活性趋势。
- 请求历史表要像专业监控工具，支持快速扫读、筛选和定位异常。

## 5. 核心调用方式

App 后端通过本机 Claude Code CLI 发起轻量请求：

```bash
claude -p \
  --no-session-persistence \
  --tools "" \
  --output-format json \
  "只回复 OK"
```

默认不额外传 token，也不强制覆盖 endpoint / model，而是直接使用本机 Claude Code 当前配置，包括通过 cc-switch 等工具切换好的 AnyRouter 配置。

可选覆盖环境变量：

```bash
ANTHROPIC_BASE_URL=https://anyrouter.top
ANTHROPIC_AUTH_TOKEN=sk-...
CLAUDE_CODE_SKIP_PROMPT_HISTORY=1
```

说明：

- 默认不使用 `--bare`，避免绕开用户现有 Claude Code 配置导致不可用。
- 默认关闭工具：`--tools ""`，防止一次探测触发任何本地工具行为。
- 使用 `--no-session-persistence`，减少探测请求带来的 CLI 侧持久化开销。
- endpoint、model、token 只是高级覆盖项，留空时不覆盖 Claude Code / cc-switch 现有配置。
- prompt 固定为极短请求，默认 `只回复 OK`。

## 6. 调度策略

### 6.1 运行窗口

默认运行时间：

```text
05:00 <= 当前本地时间 < 24:00
```

`00:00-05:00` 之间不发起请求，App 可以保持运行，但调度器睡眠到 5 点。

### 6.2 请求间隔

每次请求完成后，随机等待：

```text
60s-120s
```

间隔按每轮重新随机，不固定节奏。

### 6.3 错误处理

以下状态视为“继续抢活性”，不触发退避：

```text
429
503
524
ECONNRESET
ETIMEDOUT
overloaded
server busy
timeout
network reset
```

这些状态统一归类为：

```text
queue_miss
```

下一轮仍然在 `60-120s` 内随机触发。

### 6.4 致命错误

以下状态视为配置错误，应暂停守护并提示用户：

```text
claude 命令不存在
401
403
token 无效
model 不存在
settings 解析失败
```

这些状态统一归类为：

```text
config_error
```

### 6.5 并发控制

- 同一 profile 同一时间只允许一个请求。
- 如果上一轮仍在运行，不启动下一轮。
- 单次请求超时默认 `90s`。
- 超时后 kill 子进程，并记录为 `timeout`。
- 不做“补打”，即某次请求耗时过长后，下一轮仍从结束时间重新随机。

## 7. 状态模型

请求结果分为：

| 状态           | 含义                                | 是否继续调度 |
| -------------- | ----------------------------------- | ------------ |
| `success`      | Claude Code 返回有效响应            | 是           |
| `queue_miss`   | 429、503、524、reset、overloaded 等 | 是           |
| `timeout`      | 单次请求超过 timeout                | 是           |
| `config_error` | token、命令、模型、配置错误         | 否           |
| `paused`       | 用户手动暂停                        | 否           |
| `sleeping`     | 不在运行窗口                        | 到窗口后恢复 |

主状态展示规则：

- 最近一次为 `success`：显示 `已联通`
- 最近一次为 `queue_miss` 或 `timeout`：显示 `抢占中`
- 最近一次为 `config_error`：显示 `配置错误`
- 用户暂停：显示 `已暂停`
- 当前不在运行窗口：显示 `休眠中`

## 8. 界面设计

### 8.1 首页

首页展示当前最重要状态：

- 当前状态：`已联通 / 抢占中 / 配置错误 / 已暂停 / 休眠中`
- 当前 profile 名称
- endpoint
- model
- 最近一次成功时间
- 最近一次请求耗时
- 当前连续 queue miss 次数
- 下一次请求倒计时
- `开始 / 暂停` 按钮
- `立即 Probe` 按钮

### 8.2 24 小时活性图

使用横向 heatmap 展示最近 24 小时。

颜色建议：

- 绿色：至少一次成功
- 黄色：仅出现 `429 / queue_miss`
- 橙色：`503 / 524 / overloaded`
- 红色：配置错误
- 灰色：未运行或无数据

粒度：

- 默认每格 5 分钟。
- 最近 24 小时共 288 个点。
- 鼠标悬浮显示该时间段的成功次数、失败次数、主要错误类型。

### 8.3 请求历史

表格字段：

- 时间
- 状态
- 错误类型
- 耗时
- model
- endpoint
- stdout 摘要
- stderr 摘要

筛选能力：

- 全部
- 仅成功
- 仅 queue miss
- 仅配置错误
- 最近 1 小时
- 最近 24 小时

### 8.4 设置页

配置项：

- endpoint 覆盖项，可留空使用 Claude Code / cc-switch 当前配置
- token 覆盖项，可留空使用 Claude Code / cc-switch 当前认证
- token 类型：`ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY`，仅 token 覆盖时使用
- model 覆盖项，可留空使用 Claude Code 默认模型
- prompt
- 最小间隔
- 最大间隔
- timeout
- 运行窗口开始时间
- 运行窗口结束时间
- 开机自启
- 后台托盘运行
- 日志保留天数
- 单条 stdout/stderr 摘要最大长度

## 9. 日志与磁盘安全设计

这是本项目的重点约束。

### 9.1 防日志风暴原则

App 自己的日志系统必须防止“疯狂写盘”问题。

硬性原则：

- 不记录倒计时 tick。
- 不记录 UI render tick。
- 不记录 scheduler 空转 tick。
- 不记录重复状态心跳。
- 不因为同一种错误重复刷同一段大文本。
- 不保存无上限文本日志。
- 不在失败重试路径里同步写大日志。
- 不把 debug trace 默认写盘。

稳定运行时，写盘来源只允许是：

- probe 完成后生成的一条结构化事件。
- 配置变更。
- App 启动、退出、崩溃恢复等少量生命周期事件。
- 批量聚合后的统计数据。

目标写盘预算：

- 空闲倒计时：`0` 次写盘。
- 正常 probe：每轮最多生成 `1` 条事件。
- 稳态运行：默认每 `5` 条事件或每 `5` 分钟批量 flush 一次。
- 文本 debug 日志：默认关闭。

### 9.2 不保存完整原始输出

默认不保存完整 stdout/stderr，只保存摘要：

- stdout 最多保存 2KB
- stderr 最多保存 2KB
- token、Authorization、API key、Bearer 字段写入前脱敏
- 长输出截断并标记 `truncated`

### 9.3 低写盘策略

虽然请求间隔是 `60-120s`，但 App 仍要避免不必要写盘。

策略：

- 不记录倒计时 tick。
- 不记录 UI 状态变化 tick。
- 不做每秒日志。
- 每次 probe 只产生一条结构化事件。
- 统计数据从事件聚合计算，不额外高频落盘。
- 最近若干条实时状态可先保存在内存中。
- SQLite 写入采用批量提交：默认每 5 条或每 5 分钟 flush 一次。
- App 正常退出、暂停、系统休眠前强制 flush。
- 相同错误类型的长文本摘要只保留首条和计数，避免重复写入大段错误。
- UI 所需倒计时、当前状态、连续失败计数优先从内存状态计算。

可接受风险：

- 崩溃时可能丢失最近几条 probe 记录。
- 这比持续刷盘更符合本项目目标。

### 9.4 数据保留

日志保留是磁盘保护，不是请求限制。

默认：

- 请求历史保留 30 天。
- 单 profile 最多保留 50,000 条事件。
- 数据库建议软上限 50MB。
- 超出后按时间删除最旧记录。
- 数据库自动 vacuum 可手动触发，不默认频繁运行。

### 9.5 SQLite 建议

SQLite 配置建议：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 3000;
```

注意：

- 不使用每秒写入。
- 不把 debug trace 写入数据库。
- WAL checkpoint 不要过于频繁。
- checkpoint 只在 App 空闲、退出或 WAL 超过阈值时触发。

### 9.6 日志文件

第一版优先只用 SQLite 存结构化事件，不额外写文本日志。

如果需要文本日志：

- 默认关闭。
- 单文件最大 5MB。
- 最多保留 3 个轮转文件。
- 仅记录 App 自身错误。
- 不记录完整命令环境变量。
- 发生重复错误时做采样和合并，例如 `same error repeated 37 times`。

## 10. 安全设计

### 10.1 密钥存储

- token 存系统密钥链。
- 配置文件只存 token 引用，不存明文。
- UI 中默认隐藏 token。
- 复制、导出配置时不包含 token。

### 10.2 脱敏规则

写入日志、数据库、界面前脱敏：

```text
sk-...
Bearer ...
ANTHROPIC_AUTH_TOKEN=...
ANTHROPIC_API_KEY=...
Authorization: ...
```

示例：

```text
ANTHROPIC_AUTH_TOKEN=<redacted>
Authorization: Bearer <redacted>
```

### 10.3 命令执行

- 只执行用户配置的 `claude` 可执行文件。
- 不把 token 拼进命令行参数，避免被进程列表看到。
- token 只通过子进程环境变量传入。
- 禁止 shell 拼接，使用 Rust `Command` 参数数组。

## 11. 数据库设计

### 11.1 profiles

```sql
CREATE TABLE profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  token_ref TEXT NOT NULL,
  token_kind TEXT NOT NULL,
  model TEXT NOT NULL,
  prompt TEXT NOT NULL,
  min_interval_seconds INTEGER NOT NULL,
  max_interval_seconds INTEGER NOT NULL,
  timeout_seconds INTEGER NOT NULL,
  start_time TEXT NOT NULL,
  end_time TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### 11.2 probe_events

```sql
CREATE TABLE probe_events (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  status TEXT NOT NULL,
  error_kind TEXT,
  exit_code INTEGER,
  base_url TEXT NOT NULL,
  model TEXT NOT NULL,
  stdout_summary TEXT,
  stderr_summary TEXT,
  stdout_truncated INTEGER NOT NULL,
  stderr_truncated INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
```

### 11.3 app_state

```sql
CREATE TABLE app_state (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

## 12. 后端模块划分

```text
src-tauri/src/
  main.rs
  commands/
    profile.rs
    probe.rs
    scheduler.rs
    stats.rs
  core/
    claude_runner.rs
    classifier.rs
    redactor.rs
    scheduler.rs
    time_window.rs
  storage/
    db.rs
    migrations.rs
    event_buffer.rs
  security/
    keychain.rs
  system/
    tray.rs
    autostart.rs
```

模块职责：

- `claude_runner`：启动 `claude -p` 子进程。
- `classifier`：把 stdout/stderr/exit code 分类为状态。
- `redactor`：脱敏所有敏感内容。
- `scheduler`：运行窗口、随机间隔、单实例控制。
- `event_buffer`：内存缓冲与低频 flush。
- `keychain`：系统密钥链读写。
- `tray`：托盘菜单和后台状态。

## 13. 前端页面结构

```text
src/
  App.tsx
  pages/
    Dashboard.tsx
    History.tsx
    Settings.tsx
  components/
    StatusHero.tsx
    ActivityHeatmap.tsx
    ProbeHistoryTable.tsx
    ProfileSwitcher.tsx
    StatStrip.tsx
  lib/
    api.ts
    time.ts
    status.ts
  components/ui/
    button.tsx
    card.tsx
    badge.tsx
    table.tsx
    tabs.tsx
    dialog.tsx
    tooltip.tsx
    select.tsx
    switch.tsx
    input.tsx
```

## 14. 关键命令接口

Tauri commands：

```rust
start_scheduler(profile_id)
pause_scheduler(profile_id)
run_probe_now(profile_id)
get_current_status(profile_id)
list_probe_events(filter)
get_activity_summary(profile_id, range)
save_profile(profile)
delete_profile(profile_id)
test_claude_binary()
set_autostart(enabled)
```

## 15. MVP 范围

第一阶段必须完成：

- Tauri App 初始化。
- 单 profile 配置。
- token 存系统密钥链。
- `claude -p` 手动 probe。
- 结果分类。
- 请求历史写入 SQLite。
- 首页显示当前状态。
- 24 小时 heatmap 基础版。
- `05:00-24:00` 调度。
- `60-120s` 随机间隔。
- 低写盘 event buffer。

第一阶段可以暂缓：

- 多 profile。
- 复杂图表。
- 自动更新。

## 16. 验收标准

### 16.1 功能验收

- 用户不填写 token 时，也可以直接通过本机 Claude Code / cc-switch 当前配置手动 probe。
- endpoint、model、token 作为可选覆盖项，留空不阻止 probe。
- 用户点击开始后，App 在运行窗口内自动随机 probe。
- `429` 不会导致调度暂停。
- `401/403` 会暂停并提示配置错误。
- 最近 24 小时图能正确显示状态分布。
- 请求历史能查看最近事件。

### 16.2 安全验收

- 数据库和日志中不出现明文 token。
- App 不保存完整 stdout/stderr。
- token 不出现在进程命令行参数中。

### 16.3 磁盘验收

- 空闲倒计时时不产生写盘。
- 每次 probe 最多产生一条事件。
- 默认批量 flush，不每秒写数据库。
- 数据库有保留上限。
- 文本日志默认关闭或严格轮转。
- 连续错误场景下不会重复写入大段相同 stderr/stdout。
- 长时间运行 24 小时后，数据库和文本日志大小符合上限配置。
- 压测连续 10,000 次重复错误事件后，不出现同步刷盘、文件无限增长或 UI 卡死。

### 16.4 跨端验收

- macOS 可打包运行。
- Windows/Linux 代码路径不依赖 macOS 私有能力。
- 密钥链能力按平台适配。

## 17. 开发里程碑

### Milestone 1：CLI Probe 验证

目标：

- Rust 后端能启动 `claude -p`。
- 能传入环境变量。
- 能处理 timeout。
- 能分类成功、queue miss、config error。

产出：

- `claude_runner`
- `classifier`
- `redactor`
- 基础单元测试

### Milestone 2：Tauri MVP

目标：

- 前端能配置 profile。
- 能手动 probe。
- 能展示最近一次状态。
- 能写入 SQLite。

产出：

- Dashboard
- Settings
- SQLite migration
- Keychain 接入

### Milestone 3：守护调度

目标：

- 实现 `05:00-24:00` 时间窗。
- 实现 `60-120s` 随机调度。
- 实现暂停/恢复。
- 实现系统托盘。

产出：

- scheduler
- tray menu
- autostart 开关

### Milestone 4：可视化与历史

目标：

- 请求历史表。
- 最近 24 小时 heatmap。
- 成功率、queue miss 占比、最长未成功时间统计。

产出：

- History page
- ActivityHeatmap
- stats commands

### Milestone 5：打包与稳定性

目标：

- macOS 打包。
- Windows/Linux 打包预留。
- 日志轮转与数据库清理。
- 崩溃恢复。

产出：

- release build
- 安装包
- README

## 18. 风险与应对

| 风险         | 说明                               | 应对                                                     |
| ------------ | ---------------------------------- | -------------------------------------------------------- |
| 仍然抢不到   | AnyRouter 活跃机制不透明           | 提供统计数据，辅助调整间隔和模型                         |
| 日志写盘风暴 | 长期运行或连续错误可能触发大量写盘 | 禁止 tick 写盘、事件缓冲、批量 flush、日志采样、保留上限 |
| token 泄露   | 日志或命令行暴露密钥               | keychain、环境变量传递、脱敏                             |
| 子进程卡死   | 网关无响应                         | timeout 后 kill                                          |
| 多实例竞争   | 多个 App 同时 probe                | 单实例锁                                                 |

## 19. 默认配置

```json
{
  "base_url": "https://anyrouter.top",
  "model": "sonnet",
  "prompt": "只回复 OK",
  "min_interval_seconds": 60,
  "max_interval_seconds": 120,
  "timeout_seconds": 90,
  "start_time": "05:00",
  "end_time": "24:00",
  "stdout_summary_limit_bytes": 2048,
  "stderr_summary_limit_bytes": 2048,
  "event_flush_count": 5,
  "event_flush_interval_seconds": 300,
  "history_retention_days": 30,
  "max_events_per_profile": 50000,
  "max_database_size_mb": 50,
  "text_log_enabled": false,
  "text_log_max_file_mb": 5,
  "text_log_max_files": 3
}
```

## 20. 下一步

建议下一步直接进入 MVP 实现：

1. 初始化 Tauri + React + TypeScript 项目。
2. 实现 Rust `claude_runner`，先跑通手动 probe。
3. 实现状态分类和脱敏。
4. 接 SQLite，但先用内存 buffer 批量 flush。
5. 做首页状态和请求历史。
6. 再接入调度器和 24 小时活性图。

## 21. 参考资料

- shadcn/ui 官方文档：https://ui.shadcn.com/
- shadcn/ui Tailwind v4 文档：https://ui.shadcn.com/docs/tailwind-v4
- Tauri UI + shadcn/ui 示例生态：https://shadcn.io/template/agmmnn-tauri-ui
