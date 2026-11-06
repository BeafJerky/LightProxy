# 版本与发布规则

## 1. 发布阶段

- 当前处于 **测试版**（0.x）。不承诺稳定、兼容或数据安全，行为、状态结构与代理接管策略都可能调整。
- 进入 `1.0.0` 的条件：Windows 10 / 11 实机完成开启、关闭、切换、异常恢复、安装/升级/卸载验证；无已知高严重度问题；文档与安装说明同步。
- 用户可见位置统一写法：中文 `v0.1.3 · 测试版`，英文 `v0.1.3 (beta)`。不得出现“稳定版”“正式版”等表述。
- 0.x 期间 GitHub Release 一律标记为 **pre-release**；标签仍使用 `vX.Y.Z`。

## 2. 版本号

采用 `MAJOR.MINOR.PATCH`，固定三位数字，不出现第四位。

0.x 阶段：

- `MINOR`：新增功能、行为调整、配置或持久化结构的不兼容变更。
- `PATCH`：兼容缺陷修复、文案调整、依赖补丁、性能优化。
- `MAJOR`：仅用于 1.0.0 阶段切换，不用于 0.x 的常规迭代。

预发布后缀不写入安装包版本号，避免 Tauri / NSIS 解析差异；测试版状态由 Release 标记与界面文案表达。

规则：

- 已发布的版本号不得复用；发布后任何改动至少升 `PATCH`。
- 未发布的开发版本可以覆盖，覆盖前需确认。
- 版本号共 5 个文件，必须同时更新：
  1. `package.json`
  2. `src-tauri/tauri.conf.json5`
  3. `Cargo.toml`（`[workspace.package]`）
  4. `Cargo.lock`（`lightproxy`、`lightproxy-core` 与 `lightproxy-update`）
  5. `frontend/src/shared/version.ts`

`pnpm version:check` 校验上述 5 个文件的 7 个条目（`Cargo.lock` 覆盖 3 个 crate），任一不一致即失败。

```bash
pnpm version:bump patch    # 或 minor / major / 0.2.0
pnpm version:check
```

不得手改版本号。`state.json` 的 `schemaVersion` 与版本号相互独立，仅在持久化结构不兼容时递增。

## 3. 发布流程

1. 更新 `CHANGELOG.md`，写清变更、已知问题与影响范围。
2. 执行版本升级脚本，确认五处一致。
3. 通过全量校验：

```bash
pnpm typecheck
pnpm test
pnpm build
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
pnpm version:check
```

4. 构建 NSIS 安装包，记录 SHA256：

```bash
pnpm tauri build
```

5. 在 Windows 10 / 11 实机完成冒烟：开关代理、关闭态切换配置、退出重启恢复、开机自启、全局快捷键、托盘菜单、安装/升级/卸载。失败即不发布。
6. 提交版本变更（需明确授权）。
7. 创建附注标签并推送（需明确授权）：

```bash
git tag -a v0.1.3 -m "LightProxy 0.1.3（测试版）"
git push origin main --tags
```

8. 创建 GitHub Release，0.x 勾选 pre-release，上传安装包与校验和，附变更、已知问题、WebView2 模式与体积。
9. 同步 `README.md`、`CHANGELOG.md` 与设计文档中的版本口径。

发布后标签不可移动、不可强制覆盖；修复通过新的 `PATCH` 版本发布。

## 4. 分支与提交

- `main` 始终保持可构建；功能与修复使用短期分支，合并前通过发布门禁中的静态检查。
- Conventional Commits：`feat` / `fix` / `perf` / `refactor` / `test` / `docs` / `build` / `chore`。
- 一个提交只做一件事；不提交构建产物、日志、本地状态、凭据或签名文件。
- 推送与打标签属于对外操作，需明确授权后执行。

## 5. 测试版支持范围

- Windows 10 1809+ / Windows 11，x64。
- 只接管当前用户的 WinINET 系统代理，不写 `HKLM`、不要求管理员权限。
- HTTP / HTTPS 配置可启用；SOCKS5 当前只保存，不写入系统代理。
- 不编辑 PAC；检测到 `AutoConfigURL` 时默认拒绝启用，需用户确认后强制开启。
- 关于页提供手动检查与安装更新；不会后台轮询或静默安装，安装时应用按退出设置恢复代理。
- 更新检查依赖公开仓库的 Release Atom 源；私有仓库下检查会失败。
- 测试版可能调整状态文件结构，升级前建议保留 `state.json` 备份。

## 6. 发布说明模板

```text
LightProxy X.Y.Z（测试版）

变更
- ...

已知问题
- ...

安装
- NSIS 按用户安装，WebView2 模式：embedBootstrapper
- 安装包：LightProxy_X.Y.Z_x64-setup.exe
- SHA256：...

说明
- 当前为 0.x 测试版，功能和代理接管策略可能继续调整。
```
