# OpenDeck 多分支工作流指南

> 适配自通用模板，针对 OpenDeck (Tauri + SvelteKit + Deno) 项目定制。

---

## 一、本地仓库结构（worktree 布局）

| 目录 | 分支 | 用途 | 开发端口 |
|------|------|------|----------|
| `G:/Codes/opendeck/OpenDeck` | `main` | 最终发布，永远干净 | 无需常驻服务 |
| `G:/Codes/opendeck/OpenDeck-dev` | `dev` | 日常功能开发、集成 | Vite: 5173 |
| `G:/Codes/opendeck/OpenDeck-debug` | `debug` | 紧急 bug 修复 | Vite: 5174 |

> **初始化命令**（仅需执行一次）：
> ```bash
> # 在 main 工作树中创建 dev 和 debug worktree
> cd G:/Codes/opendeck/OpenDeck
>
> # 创建 dev worktree（基于 main）
> git worktree add ../OpenDeck-dev -b dev main
>
> # 创建 debug worktree（基于 dev）
> git worktree add ../OpenDeck-debug -b debug dev
> ```

### Tauri 开发端口说明

OpenDeck 使用 Tauri，前端通过 Vite dev server 提供 HMR，后端是 Rust 编译的原生应用。

- **dev 分支**: `deno task tauri dev`（Vite 默认 5173，Tauri 自动启动原生窗口）
- **debug 分支**: 需修改 Vite 端口避免冲突，见下方配置

**debug 分支端口配置**：在 `OpenDeck-debug/vite.config.ts` 中设置 `server.port: 5174`。
同时修改 `src-tauri/tauri.conf.json` 中 `devUrl` 为 `http://localhost:5174`。

---

## 二、远程仓库策略

| 远程 | 仓库 | 用途 |
|------|------|------|
| `origin` | GDWhisper/OpenDeck-Win | 你的 fork，日常 push/pull 所有分支 |
| `upstream` | nekename/OpenDeck | 上游原仓库，用于同步更新 |

```bash
# 已配置（当前状态）
origin    → https://github.com/GDWhisper/OpenDeck-Win.git
upstream  → https://github.com/nekename/OpenDeck.git
```

### 同步上游更新

```bash
# 在 main 工作树中拉取上游最新
cd G:/Codes/opendeck/OpenDeck
git fetch upstream
git merge upstream/main   # 或 git rebase upstream/main

# 推送到自己的 fork
git push origin main
```

---

## 三、分支合并铁律（方向与操作位置）

**代码永远从「不稳定」流向「稳定」：`debug → dev → main`**

合并必须在**接收方的工作树**里执行，严禁反向推送。

| 合并动作 | 执行位置 | 命令 |
|----------|----------|------|
| 吸收 debug 修复 | `OpenDeck-dev` | `git merge debug` |
| 吸收 dev 开发成果 | `OpenDeck` (main) | `git cherry-pick <commit>`（过滤文档提交）|
| 同步 dev 最新到 debug | `OpenDeck-debug` | `git merge dev`（仅用于拉取参考代码）|

### 合并时的文件过滤

合入 `main` 时，以下文件/目录属于开发专用，不应出现在 main 中：
- `docs/` 下的内部设计文档（bug-fixes.md 等开发记录）
- `AGENTS.md` 的开发分支专用规则（main 保留干净版）
- `.claude/` 下的本地调试配置

---

## 四、debug 分支工作流（紧急 bug 修复）

### 步骤

1. **同步最新 dev**：
   ```bash
   cd G:/Codes/opendeck/OpenDeck-debug
   git merge dev
   ```

2. **原子化提交**：
   - 核心修复 → `fix: 描述` 提交
   - 本地调试配置（端口修改、AGENTS.md 等）→ `chore: 本地调试配置` 单独提交
   - **分开提交，方便后续过滤**

3. **独立验证**：
   ```bash
   # 在 debug 工作树中启动（使用 5174 端口）
   deno task tauri dev
   ```
   不要动 `dev` 的 5173 端口实例。

4. **合入 dev**：
   ```bash
   cd G:/Codes/opendeck/OpenDeck-dev
   git merge debug
   # 解决冲突（如有），然后推送
   git push origin dev
   ```

5. **清理 debug 分支**（可选）：
   ```bash
   cd G:/Codes/opendeck/OpenDeck-debug
   git merge dev   # 同步回最新
   ```

---

## 五、dev 分支工作流（日常开发）

### 步骤

1. **直接在 `OpenDeck-dev` 中开发、提交**。

2. **提交规范**：
   - 功能/修复：`feat:` / `fix:` 前缀
   - 开发文档/配置：`docs:` / `chore:` 前缀（合入 main 时可过滤）
   - 示例：
     ```
     feat: add sleep/wake recovery for webview plugins
     docs: update bug-fixes.md with HIDAPI cache fix
     chore: add build-release.bat with MSI rename step
     ```

3. **定期吸收 debug 修复**：
   ```bash
   cd G:/Codes/opendeck/OpenDeck-dev
   git merge debug
   ```

4. **推送**：
   ```bash
   git push origin dev
   ```

---

## 六、main 分支发布（过滤开发文档）

### 原则

`main` 中不能出现 `docs:` / `chore:` 类型的开发文档或本地配置提交。

### 实现方式

**推荐：cherry-pick 功能提交**

```bash
cd G:/Codes/opendeck/OpenDeck

# 查看 dev 上的提交历史
git log dev --oneline

# 只摘取功能/修复提交，跳过 docs/chore
git cherry-pick <feat-commit-1> <fix-commit-2> ...

# 推送到两个远程
git push origin main       # 自己的 fork
git push upstream main     # 上游（如有权限）
```

**备选：交互式 rebase**

```bash
cd G:/Codes/opendeck/OpenDeck-dev
git rebase -i main   # 在 dev 上删除 docs/chore 提交
# 然后在 main 中 merge
```

### 发布检查清单

- [ ] 所有 `feat:` / `fix:` 提交已 cherry-pick 到 main
- [ ] `cargo clippy` 无警告
- [ ] `cargo fmt` 通过
- [ ] `deno task check` 和 `deno lint` 通过
- [ ] `deno task tauri build` 成功
- [ ] CHANGELOG.md 已更新
- [ ] README.md / README_zh.md 已批量更新（如需要）

---

## 七、多 Agent 协作安全守则

### 必须遵守

- **撤销已推送提交**：必须用 `git revert`，严禁 `git reset --hard` 或强制推送
- **禁止**从 debug/dev 工作树直接 `git push origin debug:dev` 或 `--force` 覆盖其他分支
- **冲突处理**：发生在哪个工作树，就在哪个工作树解决

### Worktree 安全

- 每个工作树是独立的工作目录，避免同时在多个工作树编译 Rust（共享 `target/` 会导致锁冲突）
- 如需并行编译，为 debug worktree 设置独立的 `CARGO_TARGET_DIR`：
  ```bash
  # 在 OpenDeck-debug 中
  export CARGO_TARGET_DIR=target-debug
  ```

---

## 八、各工作树 AGENTS.md 配置

### main (`OpenDeck/AGENTS.md`) — 干净版，可推公开

保持当前的 `AGENTS.md` 内容不变，仅包含：
- 架构概览
- 项目结构
- 开发命令
- 关键约定
- 已知陷阱

**不包含**：分支工作流规则、Agent 调试配置、内部设计记录。

### dev (`OpenDeck-dev/AGENTS.md`) — 开发内部

在现有 AGENTS.md 基础上，追加以下规则块：

```markdown
## 分支工作流规则

- 功能开发主分支，所有新功能在此孵化。
- 开发文档用 `docs:` 前缀，配置用 `chore:`，合入 main 时会被过滤。
- 只通过 `git merge debug` 吸收修复，禁止反向推送。
- 定期合并上游更新：`git fetch upstream && git merge upstream/main`。
```

### debug (`OpenDeck-debug/AGENTS.md`) — 临时修复

```markdown
# OpenDeck Debug 分支规则

> 仅用于紧急 bug 修复，不加新功能。

## 工作流

1. 修复前先 `git merge dev` 同步最新代码
2. 核心修复用 `fix:` 前缀提交
3. 本地调试配置（端口、AGENTS.md）用 `chore:` 单独提交
4. 独立验证后，切到 dev 工作树执行 `git merge debug`
5. 撤销用 `git revert`，禁 `reset`

## 端口配置

- Vite dev server: 5174（避免与 dev 的 5173 冲突）
- 修改 `vite.config.ts` 的 `server.port` 和 `tauri.conf.json` 的 `devUrl`

## 禁止操作

- 禁止在 debug 目录直接推送远程 dev
- 禁止添加新功能（只修 bug）
- 禁止 `git reset --hard` 或 `--force`
```

---

## 九、快速参考

### 常用命令速查

```bash
# 查看所有 worktree
git worktree list

# 创建新的 worktree
git worktree add ../OpenDeck-debug -b debug dev

# 删除 worktree（先确保分支已合并或不再需要）
git worktree remove ../OpenDeck-debug

# 在 worktree 间切换（实际是 cd 到对应目录）
cd G:/Codes/opendeck/OpenDeck          # main
cd G:/Codes/opendeck/OpenDeck-dev      # dev
cd G:/Codes/opendeck/OpenDeck-debug    # debug

# 同步上游
git fetch upstream
git merge upstream/main
```

### 提交类型速查

| 前缀 | 用途 | 合入 main |
|------|------|-----------|
| `feat:` | 新功能 | ✅ |
| `fix:` | Bug 修复 | ✅ |
| `perf:` | 性能优化 | ✅ |
| `refactor:` | 重构 | ✅ |
| `docs:` | 文档更新 | ❌ 过滤 |
| `chore:` | 构建/配置 | ❌ 过滤 |
| `test:` | 测试 | ✅ |
| `ci:` | CI 配置 | 视情况 |
