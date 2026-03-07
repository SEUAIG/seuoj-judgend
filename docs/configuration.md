# 配置系统

SEU AIJ Judge-Endpoint 使用环境变量进行配置，支持从 `.env` 文件加载。

## 环境变量

### 服务器配置

| 环境变量 | 说明 | 默认值 | 示例 |
|---------|------|--------|------|
| `AIJ_LISTEN_ADDR` | 服务器监听的 IP 地址 | `0.0.0.0` | `127.0.0.1` |
| `AIJ_LISTEN_PORT` | 服务器监听的端口号 | `9090` | `8080` |
| `AIJ_MAX_CONCURRENT_REQUESTS` | 最大并发请求处理数量 | `6` | `10` |

### 目录配置

| 环境变量 | 说明 | 默认值 | 示例 |
|---------|------|--------|------|
| `AIJ_PROBLEMS_DIR` | 题目资源文件存储目录 | `./assets/problems/` | `/var/lib/aij/problems/` |
| `AIJ_LOG_DIR` | 日志文件存储目录 | `./assets/logs/` | `/var/log/aij/` |
| `AIJ_TESTLIB_DIR` | testlib 文件存储目录 | `./assets/testlib/` | `/usr/local/share/testlib/` |

### 后端配置

| 环境变量 | 说明 | 默认值 | 示例 |
|---------|------|--------|------|
| `AIJ_BACKEND_HOST` | 后端服务器的主机地址 | `127.0.0.1` | `backend.example.com` |
| `AIJ_BACKEND_PORT` | 后端服务器的端口号 | `8080` | `80` |
| `AIJ_BACKEND_PREFIX` | 后端 API 的路径前缀 | ` `（空字符串） | `/api/v1` |

### 功能配置

| 环境变量 | 说明 | 默认值 | 示例 |
|---------|------|--------|------|
| `AIJ_SAVE_SUBMISSIONS` | 是否保存提交的代码文件 | `false` | `true` |
| `AIJ_OUTPUT_TRUNCATE_LENGTH` | 输出结果截断的字符长度 | `200` | `500` |
| `AIJ_TOOLCHAINS` | 支持的二进制工具链列表，逗号分隔 | `gcc,g++,node,java,javac,python3,go` | `gcc,g++,python3` |

## 配置文件示例

### `.env` 文件

```env
# 服务器配置
AIJ_LISTEN_ADDR=0.0.0.0
AIJ_LISTEN_PORT=9090
AIJ_MAX_CONCURRENT_REQUESTS=10

# 目录配置
AIJ_PROBLEMS_DIR=./assets/problems/
AIJ_LOG_DIR=./assets/logs/
AIJ_TESTLIB_DIR=./assets/testlib/

# 后端配置
AIJ_BACKEND_HOST=127.0.0.1
AIJ_BACKEND_PORT=8080
AIJ_BACKEND_PREFIX=

# 功能配置
AIJ_SAVE_SUBMISSIONS=false
AIJ_OUTPUT_TRUNCATE_LENGTH=200
AIJ_TOOLCHAINS=gcc,g++,node,java,javac,python3,go
```

## 配置加载顺序

1. **默认值**: 在 `config.rs` 的 `Default` 实现中定义
2. **环境变量**: 从进程环境变量读取，覆盖默认值
3. **运行时初始化**: 在 `initialize()` 函数中加载并验证配置

## 配置数据结构

### `AijConfig` 结构体

```rust
pub struct AijConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub max_concurrent_requests: usize,
    pub problems_dir: PathBuf,
    pub log_dir: PathBuf,
    pub testlib_dir: PathBuf,
    pub backend_host: String,
    pub backend_port: u16,
    pub backend_prefix: String,
    pub save_submissions: bool,
    pub output_truncate_length: usize,
    pub toolchains: Vec<String>,
    binary_path_map: Arc<RwLock<HashMap<String, PathBuf>>>,
}
```

### 配置获取方式

```rust
// 获取全局配置（单例）
let config = AijConfig::get();

// 获取后端基础地址
let backend_addr = AijConfig::get_backend_base_addr()?;

// 获取二进制工具路径
let gcc_path = AijConfig::get_binary_path("gcc").await?;
```

## 二进制工具管理

### 工具链配置

`AIJ_TOOLCHAINS` 环境变量指定系统需要检查的二进制工具，用逗号分隔。系统会在启动时检查这些工具是否在 `PATH` 中可用。

默认工具链：
- `gcc`: C 编译器
- `g++`: C++ 编译器
- `go`: Go 编译器
- `java`: Java 运行时
- `javac`: Java 编译器
- `python3`: Python 解释器
- `node`: Node.js 运行时

### 路径缓存

系统使用 `binary_path_map` 缓存二进制工具的路径，避免每次执行时都进行 `which` 查找。缓存是线程安全的，使用 `RwLock` 保护。

### 路径查找流程

1. 检查缓存中是否有该工具的路径
2. 如果未缓存，使用 `which::which()` 在 `PATH` 中查找
3. 将找到的路径存入缓存
4. 如果未找到，返回 `BINARY_NOT_FOUND` 错误

## 题目配置文件

### `problem.json` - 题目元数据

存储在题目目录 (`{problems_dir}/{pid}/problem.json`)：

```json
{
    "pid": "1",
    "description": "两数之和",
    "input": "一行两个正整数a, b($1 \\leq a, b \\leq 10^6$)。",
    "output": "一行一个正整数 $a+b$。",
    "hint": "这里是一些提示信息",
    "example": [
        {
            "in": "1 2",
            "ans": "3",
            "description": ""
        }
    ]
}
```

### `info.toml` - 题目配置

存储在题目目录 (`{problems_dir}/{pid}/info.toml`)：

```toml
[problem_info]
problem_type = "Standard"  # Standard, Interactive, Special
checker_type = "Standard"  # Standard, Special, Interactor
time_limit_ms = 1000
memory_limit_kb = 256000

[[testcases]]
id = 1
in_path = "1.in"      # 相对于 data/ 目录
ans_path = "1.ans"    # 相对于 data/ 目录
weight = 1.0
time_limit_ms = null  # 可选，覆盖全局时间限制
memory_limit_kb = null # 可选，覆盖全局内存限制

[[subtasks]]
id = 1
cases = [1]
pre_subtasks = []
score = 100
type = "min"  # min 或 sum

[custom_modules]
checker_path = "checker.cpp"     # 仅当 checker_type = "Special" 时需要
interactor_path = "interactor.cpp" # 仅当 checker_type = "Interactor" 时需要
```

## 日志配置

### 日志目录

日志文件存储在 `AIJ_LOG_DIR` 指定的目录中，按日期分割：

```
{log_dir}/
├── aij-2024-01-15.log
├── aij-2024-01-16.log
└── aij-2024-01-17.log
```

### 日志级别

日志级别通过 `RUST_LOG` 环境变量控制：

```bash
# 设置日志级别
export RUST_LOG=info
export RUST_LOG=aij_judgend=debug,tower_http=info
```

可用级别（从低到高）：
- `trace`: 最详细的日志，用于调试
- `debug`: 调试信息
- `info`: 一般信息（默认）
- `warn`: 警告信息
- `error`: 错误信息

### 日志格式

使用 `tracing` 和 `tracing-subscriber` 提供结构化日志：

```log
2024-01-15T10:30:45.123456Z INFO aij_judgend: Received judge request: submission_id=123, problem_id=1, language=Cpp
2024-01-15T10:30:45.234567Z INFO aij_judgend::judger: Judging submission 123 on test case 1
2024-01-15T10:30:45.345678Z WARN aij_judgend::fs: Binary 'g++' not found in pre-initialized map, searching in PATH
```

## Docker 配置

### Docker 环境变量

在 Docker 容器中运行时，可以通过 `-e` 选项设置环境变量：

```bash
docker run -p 9090:9090 \
  -e AIJ_LISTEN_PORT=9090 \
  -e AIJ_BACKEND_HOST=host.docker.internal \
  -e AIJ_PROBLEMS_DIR=/app/assets/problems \
  -v $(pwd)/assets:/app/assets \
  aij-judgend
```

### Dockerfile 配置

Dockerfile 中设置了以下默认值：

```dockerfile
# 设置工作目录
WORKDIR /app

# 复制构建产物
COPY target/release/aij-judgend /app/aij-judgend

# 创建必要的目录
RUN mkdir -p /app/assets/problems /app/assets/logs /app/assets/testlib

# 设置默认环境变量
ENV AIJ_PROBLEMS_DIR=/app/assets/problems
ENV AIJ_LOG_DIR=/app/assets/logs
ENV AIJ_TESTLIB_DIR=/app/assets/testlib

# 运行服务
CMD ["/app/aij-judgend"]
```

## 配置验证

### 启动时验证

服务启动时会验证以下配置：

1. **目录存在性**: 检查 `problems_dir`、`log_dir`、`testlib_dir` 是否存在，如果不存在则尝试创建
2. **二进制工具**: 检查 `toolchains` 中指定的所有二进制工具是否在 `PATH` 中可用
3. **网络可达性**: 不检查后端服务可达性（允许后端稍后启动）

### 运行时验证

运行时验证包括：

1. **题目存在性**: 验证请求的题目 ID 对应的目录是否存在
2. **配置文件**: 验证 `problem.json` 和 `info.toml` 的格式是否正确
3. **测试用例**: 验证测试用例文件是否存在
4. **自定义模块**: 验证特殊检查器和交互器是否存在（如果配置了）

## 配置热重载

当前配置不支持热重载，修改配置后需要重启服务。主要原因是：

1. 配置在启动时通过 `OnceLock` 初始化
2. 信号量等资源在启动时创建
3. 二进制工具路径在启动时缓存

如果需要动态修改配置，建议使用进程管理器（如 systemd）重启服务。