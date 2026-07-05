# GitSneaker

> 离线 Git 合并同步工具 — 把代码打包进U盘→人肉搬运→对端导入合并

> Offline Git merge sync tool — pack code to USB → carry it manually → import and merge on target

---

## 🎯 适用场景

**军工单位 · 涉密内网 · 物理隔离 · 单向导入**

- 🛡️ **物理隔离网络**：无需联网，通过 U盘物理搬运代码
- 🔒 **涉密环境**：专为军工、涉密单位设计，确保数据不出内网
- 💾 **离线同步**：两台不联网的机器之间同步 Git 仓库
- 🚫 **零网络依赖**：不依赖任何外部服务，纯本地运行
- ✅ **完整历史**：保留完整的提交历史和三方合并能力

**典型用户**：军工科研院所、涉密研发团队、物理隔离环境开发人员

---

## 功能特性

- **Bundle 导出**：增量同步，基于 sync tag 自动检测待同步提交
- **Bundle 导入**：验证 bundle 完整性，执行合并
- **冲突解决**：三栏视图（本地/共同祖先/远程），5 种自动解决模式
- **跨平台**：支持 macOS、Windows、Linux
- **离线运行**：无需网络，Petite-Vue 本地打包

## Features

- **Bundle Export**: Incremental sync, auto-detect pending commits via sync tag
- **Bundle Import**: Verify bundle integrity, execute merge
- **Conflict Resolution**: Three-column view (Local/Base/Remote), 5 auto-resolve patterns
- **Cross-platform**: macOS, Windows, Linux
- **Offline**: No network required, Petite-Vue bundled locally

---

## 🎯 Use Cases

**Military · Classified Intranet · Air-gapped Networks · One-way Import**

- 🛡️ **Physically Isolated Networks**: No internet needed, transfer code via USB
- 🔒 **Classified Environments**: Designed for military and classified organizations
- 💾 **Offline Sync**: Sync Git repos between disconnected machines
- 🚫 **Zero Network Dependency**: No external services, runs entirely locally
- ✅ **Full History**: Preserves complete commit history and 3-way merge capability

**Typical Users**: Military research institutes, classified dev teams, air-gapped environment developers

---

## 技术栈

| 层次 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 后端 | Rust (git2-rs + git CLI) |
| 前端 | Petite-Vue + HTML/CSS |
| 打包目标 | <10MB 单文件 |

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Framework | Tauri 2 |
| Backend | Rust (git2-rs + git CLI) |
| Frontend | Petite-Vue + HTML/CSS |
| Target Size | <10MB single file |

---

## 环境要求

- **Rust**：1.70+ (推荐通过 [rustup](https://rustup.rs/) 安装)
- **Node.js**：18+ (用于前端开发服务器)
- **Git**：2.25+ (bundle 功能需要)
- **Python 3**：用于开发服务器 (可选)

## Prerequisites

- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Node.js**: 18+ (for frontend dev server)
- **Git**: 2.25+ (required for bundle feature)
- **Python 3**: for dev server (optional)

---

## 快速开始

### 1. 安装依赖

```bash
# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 检查版本
rustc --version
cargo --version
```

### 2. 克隆项目

```bash
git clone <repository-url>
cd git-sneaker
```

### 3. 安装 Tauri CLI

```bash
npm install -g @tauri-apps/cli
```

### 4. 运行开发模式

```bash
# 进入 Tauri 目录
cd src-tauri

# 运行开发模式 (会自动启动前端服务器)
tauri dev
```

### 5. 构建生产版本

```bash
cd src-tauri
tauri build
```

构建产物位于 `target/release/bundle/` 目录。

### Quick Start

```bash
# Install dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
npm install -g @tauri-apps/cli

# Clone and run
git clone <repository-url>
cd git-sneaker
cd src-tauri
tauri dev
```

---

## 项目结构

```
git-sneaker/
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs               # 入口
│       ├── lib.rs                # Tauri 命令注册
│       ├── error.rs              # 错误类型定义
│       ├── engine/               # 核心引擎
│       │   ├── repo.rs           # 仓库操作
│       │   ├── commit.rs         # 提交列表
│       │   ├── bundle.rs         # Bundle 创建/验证
│       │   └── diff.rs           # Diff 生成
│       ├── merge/                # 合并引擎
│       │   ├── merge.rs          # 合并执行
│       │   ├── conflict.rs       # 冲突解析
│       │   └── auto_resolve.rs   # 自动解决
│       └── commands/             # Tauri 命令
│           ├── export.rs
│           ├── import.rs
│           └── merge_cmd.rs
├── src/                          # 前端
│   ├── index.html
│   ├── js/
│   │   ├── api.js                # API 封装
│   │   ├── app.js                # 全局状态
│   │   ├── i18n.js               # 国际化
│   │   └── views/
│   │       ├── export.js         # 导出视图
│   │       ├── import.js         # 导入视图
│   │       └── conflict.js       # 冲突解决视图
│   └── css/
│       └── style.css
└── docs/                         # 文档
```

---

## 使用流程

### 导出 Bundle

1. 打开 GitSneaker
2. 选择「导出」标签
3. 点击「浏览...」选择 Git 仓库
4. 点击「导出」查看待同步提交
5. 选择起始提交（可选增量导出）
6. 将生成的 `.bundle` 文件复制到 U盘

### 导入 Bundle

1. 将 U盘插入目标机器
2. 打开 GitSneaker
3. 选择「导入」标签
4. 选择目标仓库和 bundle 文件
5. 点击「导入」执行合并

### 解决冲突

如果合并产生冲突：
1. 自动跳转到冲突解决视图
2. 点击「全部自动解决」尝试自动解决
3. 对于无法自动解决的冲突，逐个选择解决方案
4. 点击「应用此文件」应用当前文件
5. 全部解决后点击「完成合并」

### Usage Flow

1. **Export**: Select repo → Click "Export" → Save `.bundle` to USB
2. **Import**: Select target repo + bundle → Click "Import"
3. **Conflicts**: Auto-resolve or manually select per hunk → Click "Complete Merge"

---

## 自动解决模式

| 模式 | 描述 | 置信度 |
|------|------|--------|
| 双方添加相同内容 | 两边添加相同内容 | 1.0 |
| 非重叠 | 一边为空，另一边有内容 | 1.0 |
| 单侧删除 | 一边删除，另一边未动 | 0.98 |
| 仅空白差异 | 仅空格/缩进差异 | 0.95 |
| Trailer 行 | Signed-off-by 等行冲突 | 1.0 |

## Auto-Resolve Patterns

| Pattern | Description | Confidence |
|---------|-------------|------------|
| Both-Add-Same | Both sides add identical content | 1.0 |
| Non-Overlapping | One side empty, other has content | 1.0 |
| One-Sided-Delete | One side deleted, other untouched | 0.98 |
| Whitespace-Only | Only space/indentation differences | 0.95 |
| Trailer-Lines | Signed-off-by etc. line conflicts | 1.0 |

---

## 开发

### 运行测试

```bash
cd src-tauri
cargo test
```

### 代码格式化

```bash
cargo fmt
```

### 代码检查

```bash
cargo clippy
```

## Development

```bash
cd src-tauri
cargo test     # Run tests
cargo fmt      # Format code
cargo clippy   # Lint
```

---

## 常见问题

### Q: 为什么使用 git CLI 而不是纯 git2-rs？

A: `git bundle` 格式在 libgit2 中支持不完整，使用 git CLI 可以确保兼容性。

### Q: 支持多分支吗？

A: 当前版本仅支持当前 HEAD 分支。多分支支持计划在后续版本实现。

### Q: 如何查看同步历史？

A: 同步历史存储在 sync tag 中，通过「分支」标签可查看。

## FAQ

### Q: Why use git CLI instead of pure git2-rs?

A: `git bundle` is not fully supported in libgit2. Using git CLI ensures compatibility.

### Q: Multi-branch support?

A: Current version supports only the HEAD branch. Multi-branch planned for future releases.

---

## 许可证

[MIT](LICENSE)

## License

[MIT](LICENSE)
