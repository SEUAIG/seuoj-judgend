## 题目文件构成

示例位于 [assets/problems/1](assets/problems/1) 目录下，包含以下文件：

- `description.md`: 题目描述
- `input.md`: 输入格式说明
- `output.md`: 输出格式说明
- `info.json`: 题目元信息
    - `max_cpu_time_ms`: 最大CPU时间限制，单位毫秒(可选，默认1000, -1表示无穷)
    - `max_real_time_ms`: 最大实际时间限制，单位毫秒(可选，默认2000, -1表示无穷)
    - `max_memory_byte`: 最大内存限制，单位字节(可选，默认128MB, -1表示无穷)
    - `max_stack_byte`: 最大栈内存限制，单位字节(可选，默认32MB)
    - `max_process_number`: 最大进程数(可选，默认1, -1表示无穷)
    - `max_output_size`: 最大输出大小(可选，默认1000000, -1表示无穷)
    - `test_case_number`: 测试点数量
    - `problem_type`: 题目类型，支持 "Standard"（标准题）和 "Interactive"（交互式题）
    - `checker_type`: 检查器类型，支持 "Standard"（标准检查器）和 "Special"（特殊检查器）
- `example_{num}.in`: 第num个示例输入文件
- `example_{num}.ans`: 第num个示例答案文件
- `example_{num}.md`: 第num个示例说明文件
- `{num}.in`: 测试点num的输入文件
- `{num}.ans`: 测试点num的答案文件
- `interactor`: 交互器程序（可选，仅交互式题目需要，存储形式为二进制，用户上传形式为源代码/二进制）
- `checker`: 检查器程序（可选，仅特殊检查器需要，存储形式为二进制，用户上传形式为源代码/二进制）

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

### TODO

## 环境变量说明

| 环境变量                          | 说明                           | 默认值                                  |
|-------------------------------|------------------------------|--------------------------------------|
| `AIJ_LISTEN_ADDR`             | 服务器监听的 IP 地址                 | `0.0.0.0`                            |
| `AIJ_LISTEN_PORT`             | 服务器监听的端口号                    | `9090`                               |
| `AIJ_MAX_CONCURRENT_REQUESTS` | 最大并发请求处理数量                   | `6`                                  |
| `AIJ_PROBLEMS_DIR`            | 题目资源文件存储目录                   | `./assets/problems/`                 |
| `AIJ_LOG_DIR`                 | 日志文件存储目录                     | `./assets/logs/`                     |
| `AIJ_TESTLIB_DIR`             | testlib 文件存储目录               | `./assets/testlib/`                  |
| `AIJ_BACKEND_HOST`            | 后端服务器的主机地址                   | `127.0.0.1`                          |
| `AIJ_BACKEND_PORT`            | 后端服务器的端口号                    | `8080`                               |
| `AIJ_BACKEND_PREFIX`          | 后端 API 的路径前缀                 | ` `                                  |
| `AIJ_SAVE_SUBMISSIONS`        | 是否保存提交的代码文件 (`true`/`false`) | `false`                              |
| `AIJ_OUTPUT_TRUNCATE_LENGTH`  | 输出结果截断的字符长度                  | `200`                                |
| `AIJ_TOOLCHAINS`              | 支持的二进制工具链列表，逗号分隔             | `gcc,g++,node,java,javac,python3,go` |
