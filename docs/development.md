# 开发指南

本文档为 SEU AIJ Judge-Endpoint 项目的开发者提供指导，包括开发环境设置、代码结构、开发流程和贡献指南。

## 开发环境

### 环境要求

#### 必需工具

- **Rust 工具链**: 1.70+
- **Cargo**: Rust 包管理器
- **Git**: 版本控制
- **文本编辑器/IDE**: VS Code、RustRover、或任何 Rust 支持的编辑器

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

## 代码架构

### 模块系统

#### 主要模块职责

| 模块         | 职责      | 关键结构                               |
|------------|---------|------------------------------------|
| `config`   | 配置管理    | `AijConfig`                        |
| `error`    | 错误处理    | `AijError`, `Result`               |
| `fs`       | 文件操作    | `FileSystem`                       |
| `judger`   | 评测逻辑    | `judge()`, `SupportedLanguages`    |
| `logger`   | 日志配置    | `init_logger()`                    |
| `schema`   | 数据结构    | `ProblemMetadata`, `ProblemConfig` |
| `server`   | HTTP 服务 | `app()`, `AppJson`                 |
| `server/*` | 路由处理    | 各个路由处理器                            |

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
if ! file_path.exists() {
return Err(AijError::FileSystem(
StatusCode::NOT_FOUND,
"FILE_NOT_FOUND".to_string(),
format ! ("File does not exist: {}", file_path.display()),
));
}

// 传播错误
let content = fs::read_to_string( & path).await.map_err( | e| {
AijError::FileSystem(
StatusCode::INTERNAL_SERVER_ERROR,
"READ_FILE_FAILED".to_string(),
format ! ("Failed to read file {}: {}", path.display(), e),
)
}) ?;
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

测试需要题目数据，位于 `assets/problems/p01/`：

```
assets/problems/p01/
├── problem.json      # 题目元数据
├── info.toml         # 题目配置
└── data/
    ├── 1.in          # 测试用例输入
    └── 1.ans         # 测试用例答案
```

#### 日志级别

```bash
# 设置详细日志
export RUST_LOG=debug
export RUST_LOG=aij_judgend=debug,tower_http=debug

# 运行服务
cargo run
```

### 代码风格

项目使用 rustfmt 和 clippy 强制代码风格：

```bash
# 提交前运行
cargo fmt
cargo clippy -- -D warnings
```

### CI/CD 配置

项目使用 GitHub Actions 进行持续集成，配置位于 `.github/workflows/ci.yaml`：

```yaml
name: Rust CI

on:
  push:
    branches: [ "main", "dev", "support_ci" ]
  pull_request:
    branches: [ "main", "dev" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Build and Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libseccomp-dev pkg-config

      - name: Set up Rust cache
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Lint with Clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test --verbose

  docker-check:
    name: Docker Build Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Docker Image
        run: docker build -t rust-app-test .
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