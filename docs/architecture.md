# 系统架构

## 整体架构

SEU AIJ Judge-Endpoint 采用模块化设计，主要分为以下几个层次：

1. **HTTP 接口层**: 处理 HTTP 请求和响应，定义路由和中间件
2. **业务逻辑层**: 实现评测、题目管理、文件操作等核心功能
3. **数据访问层**: 处理文件系统操作和配置读取
4. **沙箱执行层**: 通过 `judger` crate 在隔离环境中执行代码

## 模块结构

### 主要模块

```
src/
├── lib.rs                 # 库入口，定义路由和应用初始化
├── bin/main.rs           # 服务入口点，启动 HTTP 服务器
├── config.rs             # 配置管理，从环境变量读取设置
├── server.rs             # 路由定义和中间件（如并发控制信号量）
├── judger.rs             # 评测核心逻辑，处理代码编译、执行和结果判断
├── fs.rs                 # 文件系统操作，管理题目文件和临时目录
├── error.rs              # 错误类型定义
├── logger.rs             # 日志配置
├── schema.rs             # 题目配置和元数据的数据结构定义
└── server/               # HTTP 路由处理器
    ├── judge_problem_by_id.rs    # 处理 `/judge/submission` POST 请求
    ├── get_problem_by_id.rs      # 获取题目信息
    ├── edit_problem_by_id.rs     # 编辑题目
    ├── upload_problem_data.rs    # 上传题目数据
    ├── put_problem_config.rs     # 更新题目配置
    ├── get_problem_config.rs     # 获取配置源文件
    ├── get_problem_tree.rs       # 获取题目文件树
    ├── serve_problem_by_id.rs    # 题目信息服务（包含删除题目功能）
    └── serve_problem_file.rs     # 题目文件访问（包含删除文件功能）
```

### 评测模块 (`judger/`)

```
src/judger/
├── checker/              # 检查器实现
│   ├── mod.rs           # 检查器抽象
│   ├── standard.rs      # 标准检查器
│   └── special.rs       # 特殊检查器
└── utils.rs             # 工具函数（编译、权限设置等）
```

## 数据流

### 评测请求处理流程

1. **接收请求**: HTTP 服务器接收 `/judge/submission` POST 请求
2. **参数验证**: 验证提交 ID、题目 ID、代码和语言参数
3. **并发控制**: 通过信号量限制并发评测数量
4. **异步处理**: 创建异步任务处理评测，立即返回响应
5. **代码编译/解释**: 根据语言类型编译或准备解释器
6. **测试用例执行**: 对每个测试用例在沙箱中执行代码
7. **结果检查**: 根据题目类型使用相应检查器判断结果
8. **结果上报**: 将评测结果通过 HTTP 上报到后端服务
9. **资源清理**: 删除临时文件（除非配置保留）

### 配置文件加载流程

1. **读取 `problem.json`**: 从题目目录加载题目元数据
2. **读取 `info.toml`**: 从题目目录加载题目配置
3. **解析结构**: 使用 serde 将 JSON/TOML 解析为 Rust 结构体
   4**转换为 judger 配置**: 将题目配置转换为 judger crate 的配置格式

### 删除操作处理流程

1. **验证权限**: 检查题目是否存在
2. **依赖检查**: 对于文件删除，检查是否被题目配置引用
3. **安全删除**: 执行删除操作，确保数据完整性
4. **日志记录**: 记录删除操作的详细信息

## 核心数据结构

### 配置结构 (`config.rs`)

```rust
struct AijConfig {
    listen_addr: String,
    listen_port: u16,
    max_concurrent_requests: usize,
    problems_dir: PathBuf,
    log_dir: PathBuf,
    testlib_dir: PathBuf,
    backend_host: String,
    backend_port: u16,
    backend_prefix: String,
    save_submissions: bool,
    output_truncate_length: usize,
    toolchains: Vec<String>,
    binary_path_map: Arc<RwLock<HashMap<String, PathBuf>>>,
}
```

### 题目元数据 (`schema.rs`)

```rust
struct ProblemMetadata {
    pid: String,
    description: String,
    input: String,
    output: String,
    hint: String,
    example: Vec<ProblemExample>,
}

struct ProblemExample {
    r#in: String,
    ans: String,
    description: String,
}
```

### 题目配置 (`schema.rs`)

```rust
struct ProblemConfig {
    problem_info: ProblemInfo,
    testcases: Vec<TestCaseConfig>,
    subtasks: Vec<SubtaskConfig>,
    custom_modules: Option<CustomModules>,
}

struct ProblemInfo {
    problem_type: ProblemType,    // Standard, Interactive, Special
    checker_type: CheckerType,    // Standard, Special, Interactor
    time_limit_ms: i32,
    memory_limit_kb: i64,
}

struct TestCaseConfig {
    id: i32,
    in_path: String,
    ans_path: String,
    weight: f64,
    time_limit_ms: Option<i32>,
    memory_limit_kb: Option<i64>,
}

struct SubtaskConfig {
    id: i32,
    cases: Vec<i32>,
    pre_subtasks: Vec<i32>,
    score: i32,
    r#type: String,  // "min" 或 "sum"
}
```

## 错误处理

### 错误类型 (`error.rs`)

系统使用统一的错误类型 `AijError`，包含以下变体：

- `Config`: 配置相关错误
- `Logger`: 日志相关错误
- `Server`: 服务器相关错误
- `FileSystem`: 文件系统相关错误
- `Judge`: 评测相关错误
- `Request`: 请求相关错误

每个错误包含 HTTP 状态码、简短错误代码和详细错误信息。

### 错误响应

错误响应格式：

```json
{
  "code": -1,
  "message": "错误类型: 详细错误信息"
}
```

## 依赖关系

主要依赖项：

- `axum`: Web 框架（包含 multipart 支持）
- `tokio`: 异步运行时
- `judger`: 沙箱评测库
- `tracing`: 结构化日志
- `serde`: 序列化/反序列化
- `toml`: TOML 配置文件解析
- `reqwest`: HTTP 客户端（用于与后端通信）
- `zip`: ZIP 文件处理
- `tempfile`: 临时文件管理
- `tokio-util`: 异步工具函数
- `which`: 二进制工具路径查找
- `regex`: 正则表达式处理