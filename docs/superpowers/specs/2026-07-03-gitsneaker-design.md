# GitSneaker Design Spec

> 离线 Git 合并同步工具 — 把代码打包进U盘→人肉搬运→对端导入合并

## 1. 问题与定位

两台网络不通的机器需要做正常的 `git merge`，保留完整提交历史和三方合并能力。当前方案（zip 打包、patch 文件）丢失历史，冲突时只能人工逐行比对。

**核心场景**：涉密内网↔外网、工地现场↔办公室、船上↔岸上。

**参考项目对比**：

| 项目 | 技术栈 | 定位 | 开源 |
|------|--------|------|------|
| GitWand | Tauri 2 + Vue 3 | 完整 Git GUI，自动冲突解决 | MIT |
| SourceGit | C# + Avalonia UI | 完整 Git GUI，功能最全 | MIT |
| UGit | 原生 UI | 游戏行业 Git，闭源 | No |
| **GitSneaker** | **Tauri 2 + Petite-Vue + Rust** | **离线同步专用** | TBD |

GitSneaker 不做通用 Git GUI。专注一个场景：bundle 导出→人肉搬运→bundle 导入→合并。

## 2. 技术选型

| 层次 | 选择 | 理由 |
|------|------|------|
| 桌面框架 | Tauri 2 | <10MB、跨平台、Rust原生 |
| 前端 | Petite-Vue (~6KB) + 纯 HTML/CSS | 比 Svelte 更省体积，Vue 3 语法以后升级零成本 |
| Rust Git 操作 | git2-rs + `git` CLI 混合 | 基础操作用 git2-rs（结构化数据），复杂操作用 CLI（兼容性） |
| 打包目标 | <10MB 单文件 | 可直接放 U 盘运行 |

**git2-rs vs CLI 分工**：

| 操作 | 方式 | 原因 |
|------|------|------|
| revwalk / commit 列表 | git2-rs | 结构化 Commit 对象 |
| diff / tree diff | git2-rs | 结构化 Diff 输出 |
| ref / tag 读写 | git2-rs | Ref 操作稳定 |
| `git bundle create` | CLI | bundle 格式 libgit2 不完全支持 |
| `git bundle verify` | CLI | 同上 |
| `git pull <bundle>` | CLI | merge 能力完整 + merge drivers |
| `git merge --abort` | CLI | 重置冲突状态 |
| `git add` / `git commit` | CLI | 简单可靠 |

## 3. 项目结构

```
git-sneaker/
├── src-tauri/                        # Tauri 2 Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── icons/                        # 应用图标
│   └── src/
│       ├── main.rs                   # Tauri entry, 注册所有 command
│       ├── lib.rs                    # 库入口
│       ├── engine/                   # 核心 Git 引擎（无 Tauri 依赖）
│       │   ├── mod.rs
│       │   ├── repo.rs               # 仓库信息、HEAD、分支名、sync marker tag
│       │   ├── bundle.rs             # bundle export/create + verify + import
│       │   ├── diff.rs               # 结构化 diff 输出
│       │   └── commit.rs             # commit range 列表、log
│       ├── merge/                    # 合并与冲突引擎
│       │   ├── mod.rs
│       │   ├── merge.rs              # 执行 merge, 检测状态
│       │   ├── conflict.rs           # 解析冲突标记, 提取冲突块
│       │   └── auto_resolve.rs       # 5种确定性模式, 置信度评分
│       └── commands/                 # Tauri command 处理器
│           ├── mod.rs
│           ├── export.rs             # preview_export / exec_export
│           ├── import.rs             # verify_bundle / exec_import
│           └── merge_cmd.rs          # get_conflicts / auto_resolve / apply / commit
├── src/                              # 前端 (Petite-Vue)
│   ├── index.html                    # 单页应用外壳
│   ├── js/
│   │   ├── app.js                    # Petite-Vue 全局 state + 路由
│   │   ├── api.js                    # Tauri invoke() 封装
│   │   └── views/
│   │       ├── export.js             # 导出视图
│   │       ├── import.js             # 导入视图
│   │       └── conflict.js           # 冲突解决视图
│   ├── css/
│   │   └── style.css
│   └── assets/
│       └── icon.svg
└── docs/
    └── superpowers/
        └── specs/
            └── 2026-07-03-gitsneaker-design.md
```

## 4. 三层架构

```
┌──────────────────────────────────────────────────┐
│              Tauri 2 Shell (<10MB)                │
│                                                   │
│  ┌─────────────────────────────────────────────┐ │
│  │      Frontend Layer (Petite-Vue)             │ │
│  │  ┌──────────┐ ┌──────────┐ ┌─────────────┐ │ │
│  │  │ Export   │ │ Import   │ │ Conflict    │ │ │
│  │  │ View     │ │ View     │ │ Resolve     │ │ │
│  │  │          │ │          │ │ View        │ │ │
│  │  │ - 仓库   │ │ - bundle │ │ - 三栏diff  │ │ │
│  │  │ - 提交   │ │   选择    │ │ - 逐块选择  │ │ │
│  │  │   预览   │ │ - 提交   │ │ - 自动决策  │ │ │
│  │  │ - 导出   │ │   预览   │ │   确认      │ │ │
│  │  │   按钮   │ │ - 导入   │ │ - 完成      │ │ │
│  │  └──────────┘ └──────────┘ └─────────────┘ │ │
│  └─────────────────────────────────────────────┘ │
│         invoke() ↕                invoke() ↕      │
│  ┌─────────────────────────────────────────────┐ │
│  │     Rust Command Layer (Tauri handlers)      │ │
│  │  export.rs  │  import.rs  │  merge_cmd.rs    │ │
│  └─────────────────────────────────────────────┘ │
│         fn() ↕                    fn() ↕         │
│  ┌─────────────────────────────────────────────┐ │
│  │         Rust Engine Layer (Pure logic)       │ │
│  │  engine/              merge/                 │ │
│  │  ├── repo.rs          ├── merge.rs           │ │
│  │  ├── bundle.rs        ├── conflict.rs        │ │
│  │  ├── diff.rs          └── auto_resolve.rs    │ │
│  │  └── commit.rs                               │ │
│  └─────────────────────────────────────────────┘ │
│    git2-rs              std::process::Command     │
│    (revwalk, diff,      (git bundle, git merge,  │
│     refs, tags)          git pull, git commit)    │
└──────────────────────────────────────────────────┘
```

### 分层约束

1. **engine/ 和 merge/ 不能 import Tauri**——纯 Rust 库，可直接 `cargo test`
2. **commands/ 只能做参数转换**——从 Tauri State 取入参，调 engine/merge/ 函数，返回 JSON
3. **前端只能通过 `api.js` 的 `invoke()` 调后端**——不直接调 `__TAURI__`

## 5. Rust 引擎模块设计

### 5.1 engine::repo

```rust
pub struct RepoInfo {
    pub path: String,
    pub head_branch: String,
    pub head_commit: String,       // short hash
    pub last_sync: Option<SyncPoint>,
}

pub struct SyncPoint {
    pub tag_name: String,          // "sneaker-sync/main/2026-07-03T143000+0800"
    pub commit: String,
    pub timestamp: i64,
}

pub fn open(path: &Path) -> Result<RepoInfo>;
pub fn get_last_sync_tag(repo: &Repository) -> Result<Option<SyncPoint>>;
pub fn create_sync_tag(repo: &Repository) -> Result<SyncPoint>;
```

### 5.2 engine::commit

```rust
pub struct Commit {
    pub hash: String,         // short (7 char)
    pub full_hash: String,    // full sha
    pub message: String,      // 第一行
    pub author: String,
    pub date: i64,
}

pub fn list_range(repo: &Repository, from: &str, to: &str) -> Result<Vec<Commit>>;
// from = "sneaker-sync/main/*", to = "HEAD"
// git2-rs revwalk 实现
```

### 5.3 engine::bundle

```rust
pub struct BundleInfo {
    pub head_commit: String,
    pub head_message: String,
    pub commits: Vec<Commit>,     // bundle 内提交
    pub file_size: u64,
}

pub struct ExportResult {
    pub file_path: String,
    pub file_size: u64,
    pub sync_tag: String,
}

pub fn create(repo: &Path, range: &str, output: &Path) -> Result<ExportResult>;
// → git bundle create <output> <range>

pub fn verify(bundle_path: &Path) -> Result<BundleInfo>;
// → git bundle verify <path> + git bundle list-heads <path>

pub fn pull(repo: &Path, bundle_path: &Path) -> Result<()>;
// → git pull <bundle_path> <branch>
// 无冲突 → Ok, 有冲突 → 返回 Err(MergeConflict)
```

### 5.4 engine::diff

```rust
pub struct FileDiff {
    pub path: String,
    pub status: DiffStatus,
    pub hunks: Vec<Hunk>,
}

pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

pub fn diff_commits(repo: &Repository, from: &str, to: &str) -> Result<Vec<FileDiff>>;
```

### 5.5 merge::merge

```rust
pub enum MergeResult {
    Success,
    Conflicted { files: Vec<String> },
    AlreadyUpToDate,
}

pub fn pull_bundle(repo: &Path, bundle_path: &Path) -> Result<MergeResult>;
pub fn abort_merge(repo: &Path) -> Result<()>;
```

### 5.6 merge::conflict

```rust
pub struct ConflictFile {
    pub path: String,
    pub hunks: Vec<ConflictHunk>,
}

pub struct ConflictHunk {
    pub id: usize,
    pub local: String,    // OURS 版本
    pub base: String,     // 共同祖先
    pub remote: String,   // THEIRS 版本
    pub line_range: (usize, usize),
}

pub struct ResolvedHunk {
    pub hunk_id: usize,
    pub decision: HunkDecision,
    pub merged_content: String,
    pub auto: bool,
    pub confidence: f64,
}

pub enum HunkDecision {
    TakeLocal,
    TakeRemote,
    Custom(String),
}

pub fn scan_conflicts(repo: &Path) -> Result<Vec<ConflictFile>>;
pub fn apply_resolution(repo: &Path, file_path: &str, resolved: &[ResolvedHunk]) -> Result<()>;
pub fn commit_merge(repo: &Path, message: &str) -> Result<()>;
```

### 5.7 merge::auto_resolve

```rust
pub struct AutoResolveReport {
    pub resolved: Vec<(usize, ResolvedHunk)>,
    pub manual: Vec<usize>,
    pub summary: String,
}

pub fn analyze(conflicts: &[ConflictFile]) -> Result<AutoResolveReport>;
```

**五种确定性模式**（纯文本匹配，不依赖语言语义）：

| # | 模式 | 触发条件 | 决策 | 置信度 |
|---|------|---------|------|--------|
| 1 | **Both-Add-Same** | 两边 hunk 内容完全相同 | TakeLocal | 1.0 |
| 2 | **Non-Overlapping** | LOCAL 和 REMOTE 修改的行号范围不重叠 | MergeSides | 1.0 |
| 3 | **One-Sided-Delete** | 一边删除内容, 另一边未动这段 | TakeDelete | 0.98 |
| 4 | **Whitespace-Only** | 差异仅是缩进/尾空格/换行 | TakeBetterFormatted | 0.95 |
| 5 | **Trailer-Lines** | 冲突仅在 Signed-off-by/Reviewed-by 等行 | KeepBoth | 1.0 |

置信度 >= 0.95 → 自动应用（绿色），< 0.95 → 手动处理（红色）。用户始终可覆盖。
用户需点"确认应用"后才会写入文件。

## 6. Tauri Command 接口

```rust
// ====== 仓库 ======
#[tauri::command]
async fn open_repo(path: String) -> Result<RepoInfo, String>;

#[tauri::command]
async fn get_unpushed_commits(repo_path: String) -> Result<Vec<Commit>, String>;

#[tauri::command]
async fn get_last_sync(repo_path: String) -> Result<Option<SyncPoint>, String>;

// ====== 导出 ======
#[tauri::command]
async fn preview_export(repo_path: String) -> Result<ExportPreview, String>;

#[tauri::command]
async fn exec_export(repo_path: String, output_dir: String) -> Result<ExportResult, String>;
// output_dir 默认 = repo_path, 也支持 U 盘路径

// ====== 导入 ======
#[tauri::command]
async fn verify_bundle(bundle_path: String) -> Result<BundleInfo, String>;

#[tauri::command]
async fn exec_import(repo_path: String, bundle_path: String) -> Result<MergeResult, String>;

// ====== 合并 ======
#[tauri::command]
async fn get_conflicts(repo_path: String) -> Result<Vec<ConflictFile>, String>;

#[tauri::command]
async fn auto_resolve_conflicts(
    repo_path: String,
    files: Vec<ConflictFile>,
) -> Result<AutoResolveReport, String>;

#[tauri::command]
async fn apply_resolution(
    repo_path: String,
    file_path: String,
    hunks: Vec<ResolvedHunk>,
) -> Result<(), String>;

#[tauri::command]
async fn commit_merge(repo_path: String, message: Option<String>) -> Result<(), String>;

#[tauri::command]
async fn abort_merge(repo_path: String) -> Result<(), String>;
```

## 7. 前端状态与路由

Petite-Vue 单文件应用，`currentView` 做伪路由。

```javascript
// app.js - 全局状态
{
  currentView: 'export',     // 'export' | 'import' | 'conflict'

  // 仓库
  repoPath: '',
  repoInfo: null,

  // 导出
  exportPreview: null,       // { commits: [], lastSync: ... }
  exportState: 'idle',       // 'idle' | 'loading' | 'done' | 'error'

  // 导入
  bundlePath: '',
  bundleInfo: null,
  importState: 'idle',       // 'idle' | 'verifying' | 'importing' | 'conflicted' | 'done'

  // 冲突
  conflicts: [],
  selectedFile: 0,
  selectedHunk: 0,
  autoReport: null,
  resolvedDecisions: {},     // { file_path: [ResolvedHunk] }
}
```

### 页面流程

```
┌─────────┐    ┌─────────┐    ┌──────────┐
│  Export │    │ Import  │    │ Conflict │
│  View   │    │  View   │    │  View    │
├─────────┤    ├─────────┤    ├──────────┤
│1.选仓库  │    │1.选bundle│   │ ← Import │
│2.预览提交│    │2.预览提交│   │   跳转    │
│3.点导出  │    │3.点导入  │   │          │
│4.完成    │    │4.完成or  │   │1.冲突文件│
│         │    │  跳转冲突│   │  列表    │
└─────────┘    └─────────┘   │2.三栏视图│
                              │3.自动解决│
                              │  确认    │
                              │4.逐块手动│
                              │5.完成合并│
                              └──────────┘
```

## 8. 冲突视图 UI 布局

```
┌──────────────────────────────────────────────────┐
│  冲突解决                          [中止合并]     │
│  ───────────────────────────────────────────────│
│  冲突文件 (2/3)                                   │
│  ┌──────────┬──────────┬──────────────────────┐ │
│  │ src/a.js │ src/b.ts │ ...                  │ │
│  │ (已解决)  │ (2个冲突) │                      │ │
│  └──────────┴──────────┴──────────────────────┘ │
│                                                  │
│  ◀ src/b.ts — Hunk 1/2 ▶   [自动解决报告 ▼]     │
│                                                  │
│  ┌────────────┬────────────┬────────────────┐   │
│  │ LOCAL      │ BASE       │ REMOTE         │   │
│  │ (B机器)     │ (共同祖先)  │ (A机器)        │   │
│  │            │            │                │   │
│  │ function   │ function   │ function       │   │
│  │   foo() { │   foo() {  │   foo(p) {    │   │
│  │   ...     │   ...     │   ...         │   │
│  └────────────┴────────────┴────────────────┘   │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │ 合并结果 (可编辑)                          │   │
│  │ function foo(p) {                        │   │
│  │   ...                                    │   │
│  │ }                                        │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  [◀ 上一个] [▶ 下一个]  [取 LOCAL] [取 REMOTE]   │
│  [自定义编辑]     [应用此Hunk]  [全部完成]         │
└──────────────────────────────────────────────────┘
```

## 9. 错误处理

```rust
pub enum SneakerError {
    // 仓库
    RepoNotFound(String),
    NotAGitRepo(String),
    DirtyWorktree(String),

    // Bundle
    BundleCreateFailed(String),
    BundleVerifyFailed(String),
    BundleCorrupted(String),

    // Merge
    MergeFailed(String),
    MergeAborted,
    UnresolvedConflicts(Vec<String>),

    // IO
    FileNotFound(String),
    PermissionDenied(String),
    NoSpaceLeft { path: String, available: u64, needed: u64 },
}
```

- Rust: `SneakerError` → `impl Serialize` → Tauri 自动转 JSON
- 前端: `api.js` 统一 `invoke()` → `Result<T, String>`，视图层展示 toast/error bar
- 严重错误弹 modal，非严重用 confirm

## 10. 同步标记

导出时打轻量 tag：

```
格式: sneaker-sync/<branch>/<ISO8601>
示例: sneaker-sync/main/2026-07-03T143000+0800
```

多分支独立追踪：`main` 和 `develop` 各自维护同步点。

```rust
pub fn find_last_sync_tag(repo: &Repository, branch: &str) -> Option<SyncPoint>;
```

## 11. 配置存储

仓库根目录 `.sneaker.toml`:

```toml
[general]
last_bundle_dir = "/Volumes/KINGSTON"

[[sync_history]]
timestamp = "2026-07-03T14:30:00+08:00"
direction = "export"
bundle_file = "sneaker-20260703T143000.bundle"
commits_count = 5
success = true
```

不写注册表、不写 `~/.config`——完全可移植。

## 12. 构建与依赖

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
git2 = "0.19"
chrono = "0.4"
thiserror = "2"
```

体积估算：Tauri shell ~5MB + Rust ~3MB + Petite-Vue ~6KB + HTML/CSS ~20KB ≈ **<9MB**。

## 13. MVP 范围

### 第一版做

- [ ] 仓库选择 + 最近打开记录
- [ ] Export: 自动检测同步基准 + 预览提交列表 + 生成 bundle + 打 tag
- [ ] Import: 选择 bundle → 验证 → 预览 → 执行 pull
- [ ] Merge: 检测冲突状态
- [ ] Conflict: 冲突文件列表 + 三栏视图 (LOCAL/BASE/REMOTE)
- [ ] Conflict: 5种自动解决模式 + 决策报告 + 用户确认
- [ ] Conflict: 手动逐块选择 + 自定义编辑 + 完成提交
- [ ] 错误处理与提示
- [ ] `.sneaker.toml` 配置持久化

### 不做

- 多仓库批量同步
- 分支管理、commit、push/pull
- 远程仓库集成
- AI 辅助冲突解决
- MCP 服务器 / CLI 模式
- 语言语义级 diff（JSON/YAML/Markdown 格式感知）
- Excel Diff

## 14. 风险

| 风险 | 缓解 |
|------|------|
| bundle 二进制格式兼容性 | 用 git CLI 不用 libgit2 bundle |
| 大仓库 bundle 性能 | 增量 bundle（基于 sync tag range） |
| 复杂冲突逃逸自动模式 | 置信度 <0.95 不自动应用 |
| 跨平台 git 版本差异 | 要求 git >= 2.25，启动时检查 |
| 单文件 <10MB 超标 | lto=true, opt-level="z", strip=true |
