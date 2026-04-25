# SEUOJ Judgend

基于 Rust / Axum 0.8 / Tokio 的异步评测服务，使用 Seccomp 沙箱隔离用户程序。

## 构建与运行

```bash
cargo build --release
cargo test                    # 全部测试
cargo test submission         # 单个测试模块
```

依赖 `libseccomp-dev`（Linux），以及对应语言工具链（gcc、python3、javac、node、go）。

## 支持的语言

| 语言 | 枚举值 | 文件扩展名 | 备注 |
|------|--------|-----------|------|
| C | `C` | `.c` | gcc |
| C++ | `Cpp` / `Cpp11` / `Cpp17` / `Cpp20` | `.cpp` | g++，默认 C++20 |
| Python | `Python3_12` | `.py` | python3 |
| Node.js | `Nodejs22` | `.js` | node |
| Go | `Go1_22` | `.go` | go，内存限制 x2 |
| Java | `Java17` | `.java` | javac + java，时间限制 x2 |

> Python 和 Node.js 的时间限制也会 x2。

## 题目文件构成

每道题目的目录包含以下文件：

- `problem.md`: 题目描述（YAML frontmatter + Markdown）

````markdown
---
pid: "1"
---

## 题目描述

两数之和。

## 输入格式

一行两个正整数 a, b ($1 \leq a, b \leq 10^6$)。

## 输出格式

一行一个正整数 $a+b$。

## 提示

这里是一些提示信息。

## 样例

### 样例 1

#### 输入
```
1 2
```

#### 输出
```
3
```
````

支持的 H2 节名：`题目描述`/`Description`、`输入格式`/`Input`、`输出格式`/`Output`、`提示`/`Hint`、`样例`/`Examples`。

- `info.toml`: 题目测试数据的配置信息

```toml
[problem_info]
problem_type = "Standard" #（Standard：标准题，Interactive：交互题，Special：特殊检查器题）
checker_type = "Standard" # (Standard：默认检查器，Interactor：交互检查器（仅交互题），Special：特殊检查器（仅特殊检查器题）)
time_limit_ms = 1000
memory_limit_kb = 256000

[[testcases]]
id = 1
in_path = "1.in" # 输入文件路径，相对于题目目录/data
ans_path = "1.ans" # 答案文件路径，相对于题目目录/data，仅 checker_type != "Interactor" 时需要提供
weight = 100 # 测试点分值，必须为整数，且所有测试点之和必须为 100

[[subtasks]]
id = 1
cases = [1]
pre_subtasks = []
score = 100
type = "min" # (min：子任务得分为包含的测试点中得分最低的那个，sum：子任务得分为包含的测试点得分之和)

[custom_modules]
checker_path = "checker.cpp" # 仅当 checker_type = "Special" 时需要提供，路径相对于题目目录/data
interactor_path = "interactor.cpp" # 仅当 checker_type = "Interactor" 时需要提供，路径相对于题目目录/data
```

### 针对交互题的额外说明

交互器的标准输入为用户程序的标准输出，标准输出为用户程序的标准输入。
交互器的调用形式为 `./interactor <input_file> <output_file>`，其中 `<input_file>` 为 `{num}.in` 文件，
`<output_file>` 为交互器与用户程序交互时的输出文件。
交互器正常退出则认为AC，非正常退出则认为WA（错误信息取stderr）。
交互器参考[Interactors with testlib.h](https://codeforces.com/blog/entry/18455)。

### 针对特殊检查器的额外说明

特殊检查器的调用形式为 `./checker <input_file> <output_file> <answer_file>`，其中 `<input_file>` 为 `{num}.in` 文件，
`<output_file>` 为用户程序的输出文件，`<answer_file>` 为 `{num}.ans` 文件。
检查器正常退出则认为AC，非正常退出则认为WA（错误信息取stderr）。

**部分得分协议**：检查器以退出码 **7** 退出时，表示部分正确（`PartiallyAccepted`）。此时 stderr 必须以 `points` 开头，格式为 `points <score> [message]`，其中 `<score>` 为 0-100 的浮点数。例如：
```
points 75.5 部分正确：缺少边界情况
```

检查器参考[Checkers with testlib.h](https://codeforces.com/blog/entry/18431)。

### 编写交互器和检查器的额外说明

对于二进制上传方式，你需要自行编译交互器和检查器为可执行文件并上传。

**建议使用源代码上传方式**

对于源代码上传方式，建议使用 testlib 库编写交互器和检查器。另外，我们提供一个优化的 testlib
库版本，位于 [aijlib](https://github.com/SEUAIG/aijlib)，其中包含了一些常用的功能函数，你可以通过 `#include "aijlib.h"`
来使用该库。

评测机使用的编译指令为 `g++ /path/to/source.cpp -o /path/to/output -O2 -static -std=c++23 -I/path/to/testlib`。

## 环境变量说明

| 环境变量                          | 说明                           | 默认值                                  |
|-------------------------------|------------------------------|--------------------------------------|
| `AIJ_LISTEN_ADDR`             | 服务器监听的 IP 地址                 | `0.0.0.0`                            |
| `AIJ_LISTEN_PORT`             | 服务器监听的端口号                    | `9090`                               |
| `AIJ_MAX_CONCURRENT_REQUESTS` | 最大并发请求处理数量                   | `6`                                  |
| `AIJ_PROBLEMS_DIR`            | 题目资源文件存储目录                   | `./assets/problems/`                 |
| `AIJ_LOG_DIR`                 | 日志文件存储目录                     | `./assets/logs/`                     |
| `AIJ_SUBMISSIONS_DIR`         | 提交记录存储目录                     | `./assets/submissions/`              |
| `AIJ_TESTLIB_DIR`             | testlib 文件存储目录               | `./assets/testlib/`                  |
| `AIJ_BACKEND_HOST`            | 后端服务器的主机地址                   | `127.0.0.1`                          |
| `AIJ_BACKEND_PORT`            | 后端服务器的端口号                    | `8080`                               |
| `AIJ_BACKEND_PREFIX`          | 后端 API 的路径前缀                 | ` `                                  |
| `AIJ_SAVE_SUBMISSIONS`        | 是否保存提交的代码文件 (`true`/`false`) | `true`                               |
| `AIJ_OUTPUT_TRUNCATE_LENGTH`  | 输出结果截断的字符长度                  | `200`                                |
| `AIJ_TOOLCHAINS`              | 支持的二进制工具链列表，逗号分隔             | `gcc,g++,node,java,javac,python3,go` |

## API 端点

| 端点 | 说明 |
|------|------|
| `GET /health` | 健康检查 |
| `POST /judge/problem/{pid}` | 提交评测任务 |
| `GET /problem/{pid}` | 获取题目信息 |
| `PUT /problem/{pid}` | 编辑题目元数据 |
| `GET /problem/{pid}/config` | 获取题目配置（info.toml） |
| `PUT /problem/{pid}/config` | 更新题目配置 |
| `POST /problem/{pid}/upload` | 上传题目测试数据 |
| `GET /problem/{pid}/tree` | 获取题目文件树 |
| `GET /problem/{pid}/file/*path` | 获取题目文件内容 |
| `GET /submission/{sid}/file/*path` | 获取提交文件 |

评测完成后，judgend 通过 **PUT** 回调通知后端。

## 项目结构

```
src/
├── bin/main.rs           # 入口
├── lib.rs                # 路由注册与应用初始化
├── config.rs             # 环境变量配置
├── schema.rs             # 题目配置数据结构（ProblemConfig 等）
├── markdown.rs           # problem.md 解析与序列化
├── fs.rs                 # 文件系统操作
├── logger.rs             # 日志配置
├── error.rs              # 错误类型
├── server/               # HTTP 端点处理
│   ├── judge_problem_by_id.rs   # 评测逻辑
│   ├── edit_problem_by_id.rs    # 题目编辑
│   ├── get_problem_config.rs    # 获取配置
│   ├── put_problem_config.rs    # 更新配置
│   ├── serve_problem_by_id.rs   # 题目信息
│   ├── serve_problem_file.rs    # 题目文件
│   ├── serve_submission_file.rs # 提交文件
│   ├── upload_problem_data.rs   # 数据上传
│   ├── get_tree.rs              # 文件树
│   └── utils.rs                 # 工具函数
└── judger/               # 评测核心
    ├── judger.rs         # 编译与运行（Seccomp 沙箱）
    └── checker/          # 答案检查
        ├── standard.rs   # 标准对比
        └── special.rs    # 特殊检查器（含部分得分）
```
