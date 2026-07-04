# GitSneaker

> 离线 Git 合并同步工具 — 把代码打包进U盘→人肉搬运→对端导入合并

## 功能特性

- **Bundle 导出**：增量同步，基于 sync tag 自动检测待同步提交
- **Bundle 导入**：验证 bundle 完整性，执行合并
- **冲突解决**：三栏视图（LOCAL/BASE/REMOTE），5 种自动解决模式
- **跨平台**：支持 macOS、Windows、Linux
- **离线运行**：无需网络，Petite-Vue 本地打包

## 技术栈

| 层次 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 后端 | Rust (git2-rs + git CLI) |
| 前端 | Petite-Vue + HTML/CSS |
| 打包目标 | <10MB 单文件 |

## 环境要求

- **Rust**：1.70+ (推荐通过 [rustup](https://rustup.rs/) 安装)
- **Node.js**：18+ (用于前端开发服务器)
- **Git**：2.25+ (bundle 功能需要)
- **Python 3**：用于开发服务器 (可选)

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
│   │   └── views/
│   │       ├── export.js         # 导出视图
│   │       ├── import.js         # 导入视图
│   │       └── conflict.js       # 冲突解决视图
│   └── css/
│       └── style.css
└── docs/                         # 文档
```

## 使用流程

### 导出 Bundle

1. 打开 GitSneaker
2. 选择 "Export" 标签
3. 点击 "Browse" 选择 Git 仓库
4. 点击 "Preview" 查看待同步提交
5. 点击 "Export to USB" 选择输出目录
6. 将生成的 `.bundle` 文件复制到 U盘

### 导入 Bundle

1. 将 U盘插入目标机器
2. 打开 GitSneaker
3. 选择 "Import" 标签
4. 选择目标仓库和 bundle 文件
5. 点击 "Verify Bundle" 验证
6. 点击 "Import" 执行合并

### 解决冲突

如果合并产生冲突：
1. 自动跳转到冲突解决视图
2. 点击 "Auto-Resolve All" 尝试自动解决
3. 对于无法自动解决的冲突，逐个选择解决方案
4. 点击 "Apply This File" 应用当前文件
5. 全部解决后点击 "Complete Merge"

## 自动解决模式

| 模式 | 描述 | 置信度 |
|------|------|--------|
| Both-Add-Same | 两边添加相同内容 | 1.0 |
| Non-Overlapping | 一边为空，另一边有内容 | 1.0 |
| One-Sided-Delete | 一边删除，另一边未动 | 0.98 |
| Whitespace-Only | 仅空格/缩进差异 | 0.95 |
| Trailer-Lines | Signed-off-by 等行冲突 | 1.0 |

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

## 常见问题

### Q: 为什么使用 git CLI 而不是纯 git2-rs？

A: `git bundle` 格式在 libgit2 中支持不完整，使用 git CLI 可以确保兼容性。

### Q: 支持多分支吗？

A: 当前版本仅支持当前 HEAD 分支。多分支支持计划在后续版本实现。

### Q: 如何查看同步历史？

A: 同步历史存储在 `.sneaker.toml` 文件中（尚未实现）。

## 许可证

TBD

## 相关项目

- [GitWand](https://github.com/nicedoc/gitsneaker) - 完整 Git GUI
- [SourceGit](https://github.com/nicedoc/sourcegit) - 功能最全的 Git GUI
