# LightProxy

Windows 托盘上的系统代理快捷开关。基于 Tauri 2 + Rust + Vue 3 + Naive UI，单实例常驻，面板 360×560，设置页 780×640。

## 能力

- 一键开启 / 关闭 WinINET 系统代理，支持 HTTP / HTTPS / SOCKS5 配置档案
- 开关前快照原始代理设置，关闭时按快照恢复，不破坏已有配置
- PAC 冲突、代理不可达时给出明确错误，可强制开启
- 异常退出后自动恢复，代理漂移检测（2s 轮询，不自动覆盖）
- 健康检查、诊断与一键修复、修复历史
- 8 套主题（浅色 / 深色 / 跟随系统 / Win11 亮暗 / Terminal / Paper / 高对比）
- 全局快捷键、托盘菜单、开机自启

## 工程结构

```
source/
├─ crates/core/          # 纯 Rust 逻辑，不依赖 Tauri
│  ├─ model.rs           # IPC 数据模型（camelCase）
│  ├─ store.rs           # state.json 原子持久化 + .bak
│  ├─ wininet.rs         # 注册表读写与 InternetSetOption 通知
│  ├─ backend.rs         # ProxyBackend trait（可替换实现）
│  ├─ actor.rs           # 单写线程，串行化所有代理操作
│  ├─ health.rs          # TCP / HTTP 探测
│  ├─ recovery.rs        # 启动恢复与漂移处理
│  ├─ repair.rs          # 诊断与修复
│  └─ watcher.rs         # 注册表漂移监听
├─ src-tauri/            # Tauri 外壳：命令、托盘、窗口、快捷键
└─ frontend/             # Vue 3 界面（面板 + 设置）
   ├─ src/shared/api/    # 与 commands.rs 一一对应的类型化封装 + 浏览器 mock
   ├─ src/shared/theme/  # 8 套主题解析与调色板
   ├─ src/stores/app.ts  # Pinia 全局状态
   ├─ src/features/      # panel / settings / profiles / status
   └─ tests/             # Vitest
```

约束：前端不直接读写注册表，所有变更经 `src-tauri` 命令进入 core 的 actor 线程串行执行。

## 环境要求

- Windows 10 1809+（WebView2）
- Node.js ≥ 20.19，pnpm 9
- Rust 1.77+（`rust-toolchain.toml` 已固定通道与组件）
- Visual Studio Build Tools（MSVC）与 Windows SDK

## 开发

```bash
pnpm install
pnpm dev          # 仅前端，浏览器打开 http://127.0.0.1:1420，走内置 mock
pnpm tauri dev    # 完整应用（真实注册表）
```

浏览器模式通过 `src/shared/api/mock.ts` 提供内存态数据，用于纯 UI 调试；不触碰系统代理。

## 校验

```bash
pnpm typecheck
pnpm test
pnpm build
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
node scripts/verify-version.mjs
```

## 打包

```bash
pnpm tauri build
```

产物为按用户安装的 NSIS 安装包，位于 `target/release/bundle/nsis/`。图标由 `scripts/gen-icons.mjs` 生成。

## 版本号

版本号共 5 处，必须一致：

- `package.json`
- `src-tauri/tauri.conf.json5`
- `Cargo.toml`（`[workspace.package]`）
- `Cargo.lock`（`lightproxy` 与 `lightproxy-core`）
- `frontend/src/shared/version.ts`

```bash
node scripts/bump-version.mjs patch    # 或 minor / major，或显式版本如 0.2.0
node scripts/verify-version.mjs
```

## 数据位置

| 内容 | 路径 |
| --- | --- |
| 状态与配置 | `%APPDATA%\com.lightproxy.desktop\state.json`（备份 `.bak`，损坏副本 `state.corrupt-*.json`） |
| 日志与诊断导出 | `%LOCALAPPDATA%\com.lightproxy.desktop\logs` |

退出策略默认保留已开启的代理；可在设置中改为退出时关闭代理。
