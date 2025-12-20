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
    - `max_output_size`: 最大输出大小(可选，默认10000, -1表示无穷)
    - `test_case_number`: 测试点数量
    - `problem_type`: 题目类型，支持 "Standard"（标准题）和 "Interactive"（交互式题）
- `example_{num}.in`: 第num个示例输入文件
- `example_{num}.ans`: 第num个示例答案文件
- `example_{num}.md`: 第num个示例说明文件
- `{num}.in`: 测试点num的输入文件
- `{num}.ans`: 测试点num的答案文件
- `interactor`: 交互器程序（可选，仅交互式题目需要，存储形式为二进制，用户上传形式为源代码）

### 针对交互题的额外说明

交互器的标准输入为用户程序的标准输出，标准输出为用户程序的标准输入。
交互器的调用形式为 `./interactor <input_file> <output_file>`，其中 `<input_file>` 为 `{num}.in` 文件，
`<output_file>` 为交互器与用户程序交互时的输出文件。
（暂定）交互器正常退出则认为AC，非正常退出则认为WA（错误信息取stderr），所以请不要使用 `quitf(_ok, ...);` 来退出。
交互器参考[Interactors with testlib.h](https://codeforces.com/blog/entry/18455)。

### TODO

- 题目上传与检查
- 自定义判题器支持