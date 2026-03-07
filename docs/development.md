# 开发指南

本文档为 SEU AIJ Judge-Endpoint 项目的开发者提供指导，包括开发环境设置、代码结构、开发流程和贡献指南。

## 开发环境

### 环境要求

#### 必需工具
- **Rust 工具链**: 1.70+
- **Cargo**: Rust 包管理器
- **Git**: 版本控制
- **文本编辑器/IDE**: VS Code、IntelliJ Rust、或任何 Rust 支持的编辑器

#### 推荐工具
- **rust-analyzer**: Rust 语言服务器
- **cargo-watch**: 文件监视和自动重建
- **cargo-edit**: 管理依赖
- **just**: 任务运行器

### 环境设置

#### 1. 安装 Rust
```bash
# 使用 rustup 安装
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 配置环境
source "$HOME/.cargo/env"

# 安装稳定版本
rustup install stable
rustup default stable

# 安装工具链组件
rustup component add clippy rustfmt
```

#### 2. 克隆仓库
```bash
git clone https://github.com/SEUAIG/seuoj-judgend.git
cd seuoj-judgend

# 安装 Git hooks（可选）
cp .githooks/* .git/hooks/
chmod +x .git/hooks/*
```

#### 3. 安装开发依赖
```bash
# 系统依赖（Ubuntu）
sudo apt update
sudo apt install -y \
    gcc g++ python3 nodejs golang openjdk-17-jdk \
    pkg-config libssl-dev build-essential

# Cargo 工具
cargo install cargo-watch cargo-edit just
```

#### 4. 配置 IDE

**VS Code 配置**:
```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

**推荐的 VS Code 扩展**:
- `rust-lang.rust-analyzer`
- `tamasfe.even-better-toml`
- `serayuzgur.crates`
- `vadimcn.vscode-lldb`

### 项目结构

```
seuoj-judgend/
├── src/                    # 源代码
│   ├── lib.rs             # 库入口，路由定义
│   ├── bin/main.rs        # 可执行文件入口
│   ├── config.rs          # 配置管理
│   ├── error.rs           # 错误类型定义
│   ├── fs.rs              # 文件系统操作
│   ├── judger.rs          # 评测核心逻辑
│   ├── logger.rs          # 日志配置
│   ├── schema.rs          # 数据结构定义
│   ├── server.rs          # 服务器中间件
│   └── server/            # HTTP 路由处理器
├── tests/                 # 集成测试
├── assets/               # 静态资源
│   └── problems/         # 题目数据（示例）
├── docs/                 # 文档
├── Cargo.toml           # 项目配置
├── Cargo.lock           # 依赖锁文件
├── Dockerfile           # Docker 构建文件
├── .env.example         # 环境变量示例
└── README.md            # 项目说明
```

### 构建系统

#### 常用 Cargo 命令

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test
cargo test --test submission  # 运行特定测试文件

# 代码检查
cargo clippy
cargo clippy -- -D warnings   # 严格模式

# 代码格式化
cargo fmt
cargo fmt --check             # 检查格式

# 生成文档
cargo doc
cargo doc --open             # 生成并打开文档

# 依赖管理
cargo update                 # 更新依赖
cargo add <crate>            # 添加依赖
cargo rm <crate>             # 移除依赖
cargo tree                   # 查看依赖树
```

#### 使用 Just 任务运行器

**Justfile 示例**:
```makefile
# 开发任务
dev:
    cargo watch -x run

# 测试任务
test-all:
    cargo test --all-features

test-unit:
    cargo test --lib

test-integration:
    cargo test --tests

# 检查任务
check:
    cargo check
    cargo clippy -- -D warnings
    cargo fmt --check

# 构建任务
build-release:
    cargo build --release

# 清理任务
clean:
    cargo clean
```

## 代码架构

### 模块系统

#### 主要模块职责

| 模块 | 职责 | 关键结构 |
|------|------|----------|
| `config` | 配置管理 | `AijConfig` |
| `error` | 错误处理 | `AijError`, `Result` |
| `fs` | 文件操作 | `FileSystem` |
| `judger` | 评测逻辑 | `judge()`, `SupportedLanguages` |
| `logger` | 日志配置 | `init_logger()` |
| `schema` | 数据结构 | `ProblemMetadata`, `ProblemConfig` |
| `server` | HTTP 服务 | `app()`, `AppJson` |
| `server/*` | 路由处理 | 各个路由处理器 |

#### 模块依赖关系

```
main.rs → lib.rs
          ├── config
          ├── error
          ├── fs
          ├── judger
          │   ├── checker
          │   └── utils
          ├── logger
          ├── schema
          └── server
              ├── judge_problem_by_id
              ├── get_problem_by_id
              └── ...
```

### 代码规范

#### 命名约定

- **文件命名**: 使用蛇形命名法，如 `judge_problem_by_id.rs`
- **结构体命名**: 使用大驼峰命名法，如 `ProblemMetadata`
- **枚举命名**: 使用大驼峰命名法，如 `SupportedLanguages`
- **函数命名**: 使用蛇形命名法，如 `get_problem_by_id`
- **变量命名**: 使用蛇形命名法，如 `max_concurrent_requests`
- **常量命名**: 使用 screaming snake case，如 `MAX_RETRIES`

#### 注释规范

```rust
//! 模块级文档注释，描述模块功能

/// 结构体/函数文档注释，使用 Markdown 格式
///
/// # 示例
/// ```rust
/// let config = AijConfig::get();
/// ```
pub struct AijConfig {
    /// 字段文档注释
    pub listen_addr: String,

    // 普通注释，解释实现细节
    // 这个字段在初始化时缓存
    binary_path_map: Arc<RwLock<HashMap<String, PathBuf>>>,
}

/// 函数文档注释
///
/// # 参数
/// - `pid`: 题目 ID
/// - `code`: 源代码
///
/// # 返回
/// 返回评测结果或错误
///
/// # 错误
/// 如果题目不存在或配置错误，返回 `AijError`
pub async fn judge(pid: String, code: String) -> Result<JudgeResult> {
    // 实现代码
}
```

#### 错误处理

使用项目定义的 `Result<T> = std::result::Result<T, AijError>` 类型：

```rust
// 返回错误
if !file_path.exists() {
    return Err(AijError::FileSystem(
        StatusCode::NOT_FOUND,
        "FILE_NOT_FOUND".to_string(),
        format!("File does not exist: {}", file_path.display()),
    ));
}

// 传播错误
let content = fs::read_to_string(&path).await.map_err(|e| {
    AijError::FileSystem(
        StatusCode::INTERNAL_SERVER_ERROR,
        "READ_FILE_FAILED".to_string(),
        format!("Failed to read file {}: {}", path.display(), e),
    )
})?;
```

#### 异步编程

使用 `async/await` 和 Tokio 运行时：

```rust
pub async fn judge_problem_by_id(
    AppJson(payload): AppJson<JudgeRequest>,
) -> Result<impl IntoResponse> {
    // 异步操作
    let problem_config = ProblemConfig::from_pid(&payload.problem_id).await?;

    // 创建异步任务
    tokio::spawn(async move {
        let semaphore = get_judge_semaphore().await;
        let _permit = semaphore.acquire().await;

        // 评测逻辑
        let result = judge(
            payload.problem_id,
            payload.code,
            payload.language,
            payload.submission_id,
        ).await;

        // 结果上报
        report_result(&payload.submission_id, result).await;
    });

    Ok(Json(json!({"code": 0, "message": "Success"})))
}
```

### 测试策略

#### 单元测试

在模块内部使用 `#[cfg(test)]`：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_checker_str() {
        let output = "Hello\nWorld";
        let answer = "Hello\nWorld";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::Accepted
        ));
    }
}
```

#### 集成测试

在 `tests/` 目录下：

```rust
// tests/submission.rs
use aij_judgend::app;
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_judge_submission() {
    let app = app();
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "test1",
            "pid": "1",
            "code": "print(1+2)",
            "language": "Python3_12"
        }))
        .await;

    assert_eq!(response.status_code(), 200);
}
```

#### 测试数据

测试需要题目数据，位于 `assets/problems/1/`：

```
assets/problems/1/
├── problem.json      # 题目元数据
├── info.toml         # 题目配置
└── data/
    ├── 1.in          # 测试用例输入
    └── 1.ans         # 测试用例答案
```

### 调试技巧

#### 日志级别

```bash
# 设置详细日志
export RUST_LOG=debug
export RUST_LOG=aij_judgend=debug,tower_http=debug

# 运行服务
cargo run
```

#### 调试器

使用 LLDB 或 GDB 调试：

```bash
# 构建调试版本
cargo build

# 使用 lldb
lldb target/debug/aij-judgend

# 使用 gdb
gdb target/debug/aij-judgend
```

#### 性能分析

```bash
# 使用 perf
perf record -g target/release/aij-judgend
perf report

# 使用 flamegraph
cargo install flamegraph
cargo flamegraph --bin aij-judgend
```

## 开发流程

### 功能开发

#### 1. 创建功能分支

```bash
git checkout -b feat/add-new-language
```

#### 2. 实现功能

**添加新编程语言示例**:

1. 在 `src/judger.rs` 中添加语言枚举：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SupportedLanguages {
    // 现有语言...
    #[serde(rename = "rust")]
    Rust,
}
```

2. 在 `judge` 函数中添加处理逻辑：
```rust
match language {
    // 现有语言处理...
    SupportedLanguages::Rust => {
        let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
        let rustc = AijConfig::get_binary_path("rustc").await?;
        let compile_output = Command::new(rustc)
            .arg(&source_file_path)
            .arg("-o")
            .arg(&exec_path)
            .output()
            .await?;
        // 处理编译结果...
        (exec_path, vec![], judger::SeccompRuleName::CCpp)
    }
}
```

3. 更新配置文件和环境变量：
```env
AIJ_TOOLCHAINS=gcc,g++,node,java,javac,python3,go,rustc
```

#### 3. 编写测试

```rust
#[tokio::test]
async fn test_rust_language() {
    let code = r#"fn main() { println!("Hello, World!"); }"#;
    let result = judge("1", code.to_string(), SupportedLanguages::Rust, "test".to_string()).await;
    assert!(result.is_ok());
}
```

#### 4. 运行检查

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --check
cargo test
```

### Bug 修复

#### 1. 复现问题

创建最小复现用例：
```rust
#[tokio::test]
async fn test_bug_reproduction() {
    // 复现步骤
}
```

#### 2. 定位问题

使用日志和调试器定位问题根源：
```bash
RUST_LOG=debug cargo test test_bug_reproduction -- --nocapture
```

#### 3. 修复并验证

修复后运行所有相关测试：
```bash
cargo test --test submission
cargo test --lib
```

### 代码审查

#### 审查要点

1. **代码质量**
   - 符合编码规范
   - 错误处理完整
   - 注释清晰
   - 测试覆盖

2. **功能正确性**
   - 实现符合需求
   - 边界条件处理
   - 性能考虑

3. **安全性**
   - 输入验证
   - 资源管理
   - 沙箱安全

#### 审查流程

1. 创建 Pull Request
2. 等待 CI 通过
3. 请求代码审查
4. 根据反馈修改
5. 合并到主分支

## 贡献指南

### 贡献流程

1. **Fork 仓库**
2. **创建功能分支**
3. **实现功能**
4. **编写测试**
5. **提交代码**
6. **创建 Pull Request**

### 提交信息规范

使用 Conventional Commits 格式：

```
<类型>(<范围>): <描述>

[可选正文]

[可选脚注]
```

**类型**:
- `feat`: 新功能
- `fix`: bug 修复
- `docs`: 文档更新
- `style`: 代码格式
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建/工具更新

**示例**:
```
feat(judger): add Rust language support

- Add Rust to SupportedLanguages enum
- Implement compilation and execution logic
- Update toolchains configuration

Closes #123
```

### Pull Request 要求

1. **标题**: 清晰描述变更内容
2. **描述**: 详细说明变更原因和影响
3. **关联问题**: 关联相关的 Issue
4. **测试**: 包含测试用例
5. **文档**: 更新相关文档

### 代码风格

项目使用 rustfmt 和 clippy 强制代码风格：

```bash
# 提交前运行
cargo fmt
cargo clippy -- -D warnings
```

配置见 `rustfmt.toml` 和 `.clippy.toml`。

## 扩展开发

### 添加新功能模块

#### 1. 创建新模块

```rust
// src/new_module.rs
pub struct NewModule {
    // 字段定义
}

impl NewModule {
    pub async fn new_function() -> Result<()> {
        // 实现
    }
}
```

#### 2. 注册模块

在 `src/lib.rs` 中：
```rust
pub mod new_module;
```

#### 3. 添加路由

在 `src/lib.rs` 的 `app()` 函数中：
```rust
.route("/new/endpoint", get(new_module::handler))
```

### 插件系统设计

当前架构支持以下扩展点：

#### 1. 检查器插件

实现新的检查器类型：
```rust
// 1. 扩展 CheckerType 枚举
// 2. 在 checker 模块中添加实现
// 3. 在 check() 函数中添加分支
```

#### 2. 语言插件

添加新编程语言支持：
```rust
// 1. 扩展 SupportedLanguages 枚举
// 2. 在 judge() 函数中添加分支
// 3. 添加对应的沙箱规则
```

#### 3. 存储插件

替换文件系统存储（未来扩展）：
```rust
// 定义 Storage trait
pub trait Storage {
    async fn read_problem(&self, pid: &str) -> Result<ProblemMetadata>;
    async fn save_submission(&self, submission_id: &str, code: &str) -> Result<()>;
}
```

### 性能优化

#### 1. 缓存优化

```rust
// 使用 OnceCell 或 Lazy 延迟初始化
use tokio::sync::OnceCell;

static CACHE: OnceCell<HashMap<String, String>> = OnceCell::const_new();

async fn get_cached_value(key: &str) -> Result<String> {
    let cache = CACHE.get_or_init(|| async {
        // 初始化缓存
        HashMap::new()
    }).await;

    // 使用缓存
    Ok(cache.get(key).cloned().unwrap_or_default())
}
```

#### 2. 并发优化

```rust
// 使用 tokio::spawn 并行处理
let tasks: Vec<_> = testcases.iter().map(|case| {
    let case = case.clone();
    let exec_path = exec_path.clone();
    let args = args.clone();

    tokio::spawn(async move {
        judge_single_case(&case, &exec_path, &args).await
    })
}).collect();

let results = futures::future::join_all(tasks).await;
```

#### 3. 内存优化

```rust
// 使用流式处理大文件
use tokio_util::io::ReaderStream;

pub async fn process_large_file(path: &Path) -> Result<()> {
    let file = tokio::fs::File::open(path).await?;
    let stream = ReaderStream::new(file);

    tokio::pin!(stream);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        // 处理数据块
    }

    Ok(())
}
```

## 工具链

### 开发脚本

创建 `scripts/` 目录存放开发脚本：

```bash
#!/bin/bash
# scripts/setup_dev.sh

set -e

echo "Setting up development environment..."

# 安装 Rust
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 安装工具
echo "Installing tools..."
cargo install cargo-watch cargo-edit just

# 安装系统依赖
if command -v apt &> /dev/null; then
    sudo apt update
    sudo apt install -y gcc g++ python3 nodejs golang openjdk-17-jdk
elif command -v yum &> /dev/null; then
    sudo yum install -y gcc gcc-c++ python3 nodejs golang java-17-openjdk-devel
fi

echo "Development environment setup complete!"
```

### CI/CD 配置

GitHub Actions 配置示例 (`.github/workflows/ci.yml`):

```yaml
name: CI

on:
  push:
    branches: [ main, dev ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: clippy, rustfmt

    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y gcc g++ python3 nodejs golang openjdk-17-jdk

    - name: Check format
      run: cargo fmt --check

    - name: Clippy check
      run: cargo clippy -- -D warnings

    - name: Run tests
      run: cargo test --all-features

    - name: Build release
      run: cargo build --release
```

## 文档维护

### 代码文档

使用 `cargo doc` 生成 API 文档：

```bash
# 生成文档
cargo doc --no-deps

# 本地查看
cargo doc --open
```

### 用户文档

用户文档位于 `docs/` 目录，使用 Markdown 格式：

- `README.md`: 项目概述
- `architecture.md`: 系统架构
- `api.md`: API 参考
- `configuration.md`: 配置说明
- `problem-structure.md`: 题目文件结构
- `judging-process.md`: 评测流程
- `checkers.md`: 检查器系统
- `deployment.md`: 部署指南
- `development.md`: 开发指南（本文档）
- `testing.md`: 测试说明

### 更新文档

当代码变更时，需要更新相关文档：

1. **API 变更**: 更新 `docs/api.md`
2. **配置变更**: 更新 `docs/configuration.md`
3. **功能变更**: 更新相关功能文档
4. **示例更新**: 更新示例代码

## 故障排除

### 常见问题

#### 1. 编译错误

**问题**: `cannot find derive macro`
**解决**: 确保依赖版本正确，运行 `cargo update`

#### 2. 测试失败

**问题**: 测试找不到题目数据
**解决**: 确保 `assets/problems/1/` 目录存在并包含示例数据

#### 3. 权限错误

**问题**: 沙箱执行失败
**解决**: 确保有 `CAP_SYS_ADMIN` 能力或以 root 运行

#### 4. 性能问题

**问题**: 评测速度慢
**解决**: 检查系统资源，调整并发设置

### 调试资源

- **日志文件**: `assets/logs/aij-*.log`
- **系统日志**: `journalctl -u aij-judgend`
- **性能分析**: 使用 `perf` 或 `flamegraph`
- **内存分析**: 使用 `valgrind` 或 `heaptrack`

### 获取帮助

- **Issue 跟踪**: GitHub Issues
- **讨论区**: GitHub Discussions
- **代码审查**: Pull Request 评论
- **文档**: 项目文档和 Rust 文档

## 发布流程

### 版本管理

使用语义化版本号：`主版本.次版本.修订版本`

- **主版本**: 不兼容的 API 变更
- **次版本**: 向后兼容的功能添加
- **修订版本**: 向后兼容的问题修复

### 发布步骤

1. **更新版本号**
   ```bash
   # Cargo.toml
   [package]
   version = "0.2.0"
   ```

2. **更新 CHANGELOG**
   ```markdown
   ## [0.2.0] - 2024-01-15

   ### Added
   - 添加 Rust 语言支持
   - 添加新的检查器类型

   ### Changed
   - 改进错误处理
   - 优化性能

   ### Fixed
   - 修复内存泄漏问题
   ```

3. **创建发布标签**
   ```bash
   git tag -a v0.2.0 -m "Release version 0.2.0"
   git push origin v0.2.0
   ```

4. **构建发布包**
   ```bash
   cargo build --release
   tar -czf aij-judgend-v0.2.0-x86_64-linux.tar.gz \
     -C target/release aij-judgend \
     README.md LICENSE
   ```

5. **发布到 crates.io**（可选）
   ```bash
   cargo publish
   ```

### 向后兼容性

保持 API 向后兼容性：
- 不删除或重命名公共 API
- 不改变现有行为（除非修复 bug）
- 使用特性标志控制实验性功能

## 附录

### 开发检查清单

- [ ] 代码符合编码规范
- [ ] 包含单元测试
- [ ] 包含集成测试
- [ ] 通过所有检查（fmt, clippy）
- [ ] 更新相关文档
- [ ] 性能测试通过
- [ ] 安全审查通过

### 常用命令速查

```bash
# 开发
cargo watch -x run              # 开发模式运行
cargo watch -x test             # 测试模式

# 检查
cargo fmt --check              # 格式检查
cargo clippy -- -D warnings    # 代码检查
cargo audit                    # 安全审计

# 测试
cargo test                     # 所有测试
cargo test -- --nocapture      # 显示输出
cargo test test_name           # 单个测试

# 文档
cargo doc --open              # 生成文档
cargo doc --document-private-items  # 包含私有项

# 发布
cargo build --release         # 发布构建
cargo publish --dry-run       # 发布测试
```

### 资源链接

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Tokio 文档](https://tokio.rs/)
- [Axum 文档](https://docs.rs/axum)
- [项目 GitHub](https://github.com/SEUAIG/seuoj-judgend)
- [在线文档](https://seuaig.github.io/seuoj-judgend/)