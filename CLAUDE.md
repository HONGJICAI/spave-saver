# CLAUDE.md

Space-Saver（硬盘节省大师）：Rust + Tauri + SvelteKit 的磁盘空间清理工具。

## 架构

Rust workspace + Tauri 应用：

- `crates/core` — 核心算法（文件扫描、BLAKE3 去重、感知哈希图片相似度、压缩插件）
- `crates/service` — 服务层（任务调度、API、文件操作、进度上报）
- `crates/db` — SQLite 与缓存
- `crates/utils` — 通用工具（配置、日志、错误、时间）
- `crates/cli` — 命令行入口
- `app/src-tauri` — Tauri 后端；所有命令定义在 `src/commands.rs`，注册在 `src/lib.rs` 的 `invoke_handler`
- `app/src` — SvelteKit 前端（Svelte 5 + Tailwind 4 + Flowbite），测试用 Vitest + Testing Library

前端有两种运行模式，由 **`app/src/lib/api/index.ts`（统一 API 层）** 路由：

- **Tauri 模式**：通过 `invoke` 调真实 Rust 后端
- **Web 模式**（`pnpm dev:web`，部署到 GitHub Pages）：走 `app/src/mock/` 的 mock 数据

组件代码**只允许**通过 `lib/api` 调后端，禁止在组件里直接 `invoke`。

## 常用命令

后端（仓库根目录）：

```bash
cargo fmt --all                                          # 格式化
cargo fmt --all -- --check                               # 格式检查（CI 同款）
cargo clippy --all-targets --all-features -- -D warnings # lint（CI 同款，warning 即失败）
cargo test --all                                         # 全部测试（CI 同款）
cargo test -p space-saver-core                           # 单 crate 测试
```

前端（`app/` 目录，用 pnpm）：

```bash
pnpm check           # 类型检查 svelte-check（CI 同款）
pnpm test -- --run   # 单次跑全部测试（CI 同款）
pnpm test            # watch 模式
pnpm dev:web         # Web 模式启动（mock 数据，无需 Tauri）
```

## 开发规则

本项目纯云端开发，**CI 是唯一的质量门禁**，没有"本地手动验证"环节。所以：

### 1. 推送前必须本地跑通 CI 同款检查

每次 push 前依次执行（与 `.github/workflows/ci.yml` 完全一致）：

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. `cd app && pnpm check && pnpm test -- --run`

任何一步失败就先修复再推。不要靠 CI 发现格式/clippy 问题。

### 2. 后端改动必须带测试

后端是质量重心，测试要求从严：

- **新增/修改任何 public 函数或 Tauri command，必须同步新增/更新测试**。测试放在同文件的 `#[cfg(test)] mod tests`（项目现有惯例），异步用 `#[tokio::test]`。
- 必须覆盖：正常路径、**错误路径**（不存在的路径、权限失败、空输入、空列表）、边界值（空文件、单元素、阈值边界如 `threshold` 的 0/1）。
- 涉及文件系统的测试用 `tempfile` 临时目录构造，禁止依赖真实磁盘的固定路径或修改仓库内文件。
- 修 bug 时先写一个能复现该 bug 的失败测试，再修复。
- 不允许为通过 CI 而删除或 `#[ignore]` 测试；确需忽略要写明原因。

### 3. 前端 mock 必须完整覆盖后端能力（最重要）

Web 模式（GitHub Pages 演示）是前端的主要验证途径，**mock 的目标是：后端能做到的每个功能、每种状态，Web 模式都能演示出来**。

新增或修改 Tauri command 时，以下四处必须**同一个 PR 内**同步完成：

1. `app/src-tauri/src/commands.rs` 定义 + `app/src-tauri/src/lib.rs` 的 `invoke_handler` 注册
2. `app/src/lib/api/index.ts` 增加封装函数，包含 `isTauri` 真实分支和 Web mock 分支
3. mock 数据：较大的 mock 放 `app/src/mock/` 单独文件，简单的可内联在 API 层
4. `app/src/lib/types.ts` / API 层的 TS 类型与 Rust 返回结构同步

mock 数据本身的要求：

- **不只 mock 成功**：每种后端可能返回的状态（成功/失败/跳过/部分失败）都要能在 Web 模式触发。沿用现有的"路径关键字触发状态"惯例：
  - 路径含 `locked` → 权限失败
  - 路径含 `already-tiny` → 压缩 skipped（产物不更小，并记入 web 模式的 skip cache）
  - 路径含 `usb-drive` → trash 模式失败、permanent 成功
  - 路径含 `missing` → 压缩 failed（File not found）
  - 路径含 `empty-dir` → 各扫描类接口返回空结果（演示空状态 UI）
  - 新增状态时按同样模式扩展，并在 mock 代码注释里说明触发词
- 有状态的后端行为 mock 也要有状态：如 skip cache（`app/src/mock/skipCache.ts`）在 web 模式是真实的内存状态——压缩 skipped 会记录条目、下次扫描以 cached result 理由排除该文件、清除后恢复，完整复刻后端闭环
- mock 的错误信息措辞要贴近后端真实返回（如 `Permission denied (os error 13)`），保证 UI 错误展示路径被真实地测到
- 异步操作的 mock 用 `setTimeout` 加少量延迟，让 loading 状态可见
- API 层每个函数在 `app/src/lib/api/index.test.ts` 都要有测试

**契约测试**：`app/src/lib/api/contract.test.ts` 会解析 `commands.rs`、`lib.rs` 和 `api/index.ts`，强制三方一致（command 已定义 ↔ 已注册 ↔ API 层有封装）。漏注册、漏封装或 invoke 名字打错都会在 CI 挂掉，不要通过放宽该测试来绕过，应补齐缺口。

### 4. 前后端类型契约

- Rust serde 序列化字段为 `snake_case`，TS interface 字段保持一致（如 `total_files`、`original_size`），不要在 TS 侧改成 camelCase。
- Rust 侧 `Option<T>` 对应 TS 侧可选字段 `field?: T | null`。
- 改 Rust 返回结构时，必须同步改 TS 类型、mock 数据和相关测试。

### 5. 提交约定

- 提交信息用英文祈使句，说明做了什么和为什么。
- 一个 PR 内保持后端、API 层、mock、测试的原子性——不要先合后端再"以后补 mock"。

## Definition of Done

一个改动完成的标准：

- [ ] 后端逻辑有测试（含错误路径）
- [ ] Tauri command ↔ API 层 ↔ mock ↔ TS 类型四处同步
- [ ] Web 模式（`pnpm dev:web`）能演示该功能的所有状态
- [ ] 本地跑通全部 CI 同款检查（fmt / clippy / cargo test / pnpm check / pnpm test）
