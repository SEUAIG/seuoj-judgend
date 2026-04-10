## 题目文件构成

示例位于 [assets/problems/test01](assets/problems/test01) 目录下，包含以下文件：

- `problem.json`: 题目元数据信息

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

- `info.toml`: 题目测试数据的配置信息

```toml
[problem_info]
problem_type = "Standard" #（Standard：标准题，Interactive：交互题，Special：特殊检查器题）
checker_type = "Standard" # (Standard：默认检查器，Interactor：交互检查器 （仅交互题），Custom：自定义检查器（仅特殊检查器题）)
time_limit_ms = 1000
memory_limit_kb = 256000

[[testcases]]
id = 1
in_path = "1.in" # 输入文件路径，相对于题目目录/data
ans_path = "1.ans" # 答案文件路径，相对于题目目录/data，仅 checker_type != "Interactor" 时需要提供
weight = 1.0

[[subtasks]]
id = 1
cases = [1]
pre_subtasks = []
score = 100
type = "min" # (min：子任务得分为包含的测试点中得分最低的那个，sum：子任务得分为包含的测试点得分之和)

[custom_modules]
checker_path = "checker.cpp" # 仅当 checker_type = "Custom" 时需要提供，路径相对于题目目录/data
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
