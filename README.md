# 🚦 Copilot Router

一个基于 **Tauri + Rust + React** 的本地 AI 路由器。  
目标是把 **Claude Code / Codex / 其他兼容客户端** 统一接到 GitHub Copilot 能力，并提供可视化控制台。

## ✨ 功能亮点

- 🔐 GitHub Device Flow 登录 + Token 刷新
- 🌐 本地代理转发：`/v1/chat/completions`、`/v1/messages`、`/v1/models`
- 🧠 按客户端独立模型配置（Claude Code / Codex / 通用）
- 🧩 细粒度模型位（如 Claude 的 Haiku/Sonnet/Opus/Reasoning/Fast）
- 🛠️ Claude 接管状态检测与一键修复（`~/.claude/settings.json`）
- 📊 Dashboard 统计：请求量、Token 消耗、模型分布、日志
- 🌗 主题切换：白天 / 晚上

## 🧱 技术栈

- Frontend: React + TypeScript + Vite + Tailwind CSS + React Query + Recharts
- Backend: Tauri 2 + Rust + Axum + Reqwest + SQLx (SQLite)

## 🚀 快速开始

### 1) 环境要求

- Node.js 18+
- Rust stable
- Tauri 2 构建依赖（请按官方文档安装）

### 2) 安装依赖

```bash
npm install
```

### 3) 启动开发

```bash
npm run tauri:dev
```

### 4) 构建检查

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

## 🔌 客户端接入说明

本地代理默认地址：

- Base URL: `http://127.0.0.1:3100/v1`
- API Key: `copilot-router`（大多数 SDK 会校验非空）

## 🧭 模型配置页面

模型页已按客户端拆分：

- `#/mappings/claude` → Claude Code
- `#/mappings/codex` → Codex
- `#/mappings/generic` → 通用插件

每页单独保存对应客户端的模型配置，更直观。

## 🖼️ 界面预览

> 可将截图放到 `docs/screenshots/` 目录并替换下面链接。

### Dashboard

![Dashboard](docs/screenshots/dashboard.png)

### Model Mappings

![Model Mappings](docs/screenshots/model-mappings.png)

## 🛡️ Claude 接管

应用会读写并校验：`~/.claude/settings.json`

关键字段包括：

- `ANTHROPIC_BASE_URL`
- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `ANTHROPIC_MODEL` / `ANTHROPIC_DEFAULT_*_MODEL`

当 Dashboard 显示“未命中本地代理”时，可点击「一键修复接管」。

## 📁 项目结构

```text
src/            # React 前端
src-tauri/      # Rust 后端与 Tauri 配置
```

## 📦 开源发布建议

1. 🔎 发布前检查仓库中是否包含真实 Token/密钥。
2. ✅ 执行 `npm run build` + `cargo check`。
3. 🏷️ 使用 GitHub Release 发布多平台产物。
4. 📝 在 Release Notes 说明支持客户端与已知限制。

## 🗺️ Roadmap

- [ ] 更细粒度的客户端识别（基于请求特征而非仅模型名）
- [ ] 模型路由规则可视化调试（命中路径追踪）
- [ ] 导入/导出配置（含模板预设）
- [ ] 更多可观测指标（错误类型分布、耗时分位）
- [ ] 可选的自动更新与版本迁移提示

## ❓FAQ

### URL 填了还要填 Key 吗？

建议要填。很多 SDK/插件要求 Key 非空，统一填：`copilot-router`。

### 为什么代理开启后仍然失败？

优先检查：

- Token 是否有效
- 代理开关是否已开启
- Base URL 是否指向本地 `/v1`
- Claude 接管是否命中本地代理

### 为什么显示模型和最终请求模型不同？

这是预期行为。系统支持“展示名”与“实际请求 model id”分离，便于多客户端兼容。

## 📄 License

MIT
