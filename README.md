# Copilot Router

一个基于 Tauri + Rust + React 的本地 AI 路由器，用来把 Claude Code、Codex 和各类 OpenAI/Anthropic 兼容客户端统一接入 GitHub Copilot 认证能力，并提供可视化状态面板、模型映射与用量追踪。

## 功能特性

- GitHub Device Flow 登录与 Token 刷新
- 本地代理转发（`/v1/chat/completions`、`/v1/messages`、`/v1/models`）
- 三类客户端模型目标配置（Claude Code / Codex / 通用）
- 模型映射规则（显示名与请求名分离）
- Claude 接管状态检测与一键修复（`~/.claude/settings.json`）
- 使用统计、模型分布、请求日志面板
- 简约深色主题 + 午夜主题切换
- 第三方插件配置示例一键复制

## 技术栈

- 前端：React + TypeScript + Vite + Tailwind CSS + React Query + Recharts
- 后端：Tauri 2 + Rust + Axum + Reqwest + SQLx (SQLite)

## 快速开始

### 环境要求

- Node.js 18+
- Rust stable
- Tauri 2 构建依赖（按官方文档安装）

### 安装依赖

```bash
npm install
```

### 开发运行

```bash
npm run tauri:dev
```

### 前端构建

```bash
npm run build
```

### Rust 检查

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 第三方客户端接入

本地代理默认地址：

- Base URL: `http://127.0.0.1:3100/v1`
- API Key: `copilot-router`（多数客户端要求非空，填这个即可）

> 也可以直接在 Dashboard 的“第三方插件配置示例”中一键复制模板。

## Claude 接管

应用会读取并可修复以下配置：

- 文件：`~/.claude/settings.json`
- 关键字段：`ANTHROPIC_BASE_URL`、`ANTHROPIC_API_KEY`、`ANTHROPIC_AUTH_TOKEN`

如果 Dashboard 显示“Claude 接管未命中”，可点击“一键修复接管”。

## 项目结构

```text
src/            # React 前端
src-tauri/      # Rust 后端与 Tauri 配置
```

## 开源发布建议

1. 在发布前确认本地未包含真实 Token 或敏感配置。
2. 使用 `cargo check` + `npm run build` 做发布前验证。
3. 使用 GitHub Release 上传平台构建产物（macOS/Windows/Linux）。
4. 在 Release Notes 里标注支持的客户端与已知限制。

## 常见问题

### 1) 只填 URL 不填 Key 可以吗？

通常不行。很多 SDK/插件会校验 Key 非空。建议统一填写：`copilot-router`。

### 2) 为什么开启代理后请求仍失败？

请优先检查：

- Dashboard 中 Token 状态是否有效
- 代理开关是否开启
- 客户端 Base URL 是否指向本地 `/v1`
- Claude 接管状态是否命中本地代理

### 3) 显示模型和真实请求模型不一致？

这是预期行为。项目支持“展示 label + 实际请求 value(id)”模式，便于兼容不同客户端显示与后端路由。

## License

MIT
