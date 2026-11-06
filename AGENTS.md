# AGENTS.md

LightProxy：Windows 系统代理托盘开关。Tauri 2 + Rust workspace + Vue 3 / Naive UI / Pinia。产品与设计文档在仓库外 `../docs/`。

## 命令

```bash
pnpm install
pnpm dev                 # 纯前端 + 内置 mock
pnpm tauri dev           # 完整应用
pnpm typecheck
pnpm test
pnpm build
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
pnpm version:check
```

## 结构边界

- `crates/core`：纯 Rust，不得依赖 Tauri 或前端。注册表、持久化、状态机、探测、恢复、诊断都在这里。
- `src-tauri`：只做应用装配、IPC 命令、托盘、窗口、全局快捷键。
- `frontend`：只经 `src/shared/api` 调用命令；禁止直接读写注册表或自行实现代理逻辑。

## 硬约束

- 所有代理变更必须经 `ActorHandle` 单线程串行执行；不得在命令层直接写 HKCU。
- 开启前保存快照，关闭时按快照恢复；未知代理配置不得覆盖。PAC 冲突默认拒绝，除非用户显式强制。
- IPC 数据结构 serde `camelCase`；Tauri 命令与参数保持现有命名，前端类型在 `shared/api/types.ts` 同步。
- 新增或修改命令时，同时更新 `shared/api/index.ts`、`shared/api/mock.ts` 与 Rust 端。
- IPC 错误统一 `CoreError { code, stage, message, hint }`；面向用户的文案用中文，可附修复提示。
- 主题 token 变更必须同时改 `styles/tokens.css` 与 `shared/theme/themes.ts`，`pnpm test` 会校验一致性。
- 版本号 5 处联动，用 `pnpm version:bump <patch|minor|major|x.y.z>`，不要手改。
- 不引入数据库、本地 HTTP 服务、Node sidecar 或 vue-router；窗口只有一个 WebView，面板与设置页靠视图切换。
- 代码注释只写非显然的原因；界面文案中文，标识符英文；不使用 emoji。

## 验证要求

提交前至少通过：`pnpm typecheck`、`pnpm test`、`pnpm build`、`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`、`pnpm version:check`。涉及 UI 的改动需在 mock 模式下实际打开面板与设置页核对明暗主题。
