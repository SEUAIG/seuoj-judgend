# 检查器系统

SEU AIJ Judge-Endpoint 支持三种类型的检查器：标准检查器、特殊检查器和交互检查器。本文档详细说明各种检查器的实现和使用方法。

## 检查器类型概述

| 检查器类型 | 适用题目类型 | 检查方式   | 输入文件 | 输出文件 | 答案文件 |
|-------|--------|--------|------|------|------|
| 标准检查器 | 标准题    | 逐行比较输出 | 不需要  | 用户输出 | 期望答案 |
| 特殊检查器 | 特殊检查器题 | 外部程序检查 | 测试输入 | 用户输出 | 期望答案 |
| 交互检查器 | 交互题    | 交互器程序  | 测试输入 | 交互输出 | 不需要  |

## 标准检查器

### 设计原理

标准检查器通过逐行比较用户程序的输出和期望答案来判断结果。它进行了适当的规范化处理，使检查更加灵活。

### 规范化规则

1. **去除末尾空行**: 移除输出末尾的所有空行
2. **修剪行尾空白**: 移除每行末尾的空格和制表符
3. **保留行首空白**: 不修改行首的空白字符
4. **保留空行**: 中间的空行保留

### 比较算法

```rust
fn standard_checker_str(output: &str, answer: &str) -> CheckerResult {
    // 1. 规范化处理
    let normalize = |text: &str| {
        text.lines()
            .map(|line| line.trim_end())  // 修剪行尾空白
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end_matches(['\n', '\r'])  // 去除末尾空行
    };

    let output_normalized = normalize(output);
    let answer_normalized = normalize(answer);

    // 2. 按行分割
    let output_lines: Vec<&str> = output_normalized.lines().collect();
    let answer_lines: Vec<&str> = answer_normalized.lines().collect();

    // 3. 逐行比较
    if output_lines == answer_lines {
        CheckerResult::Accepted
    } else {
        // 4. 找到第一个不同的行
        for i in 0..output_lines.len().max(answer_lines.len()) {
            let out_line = output_lines.get(i).unwrap_or(&"");
            let ans_line = answer_lines.get(i).unwrap_or(&"");
            if out_line != ans_line {
                return CheckerResult::WrongAnswer(format!(
                    "In line {}:\nExpected: '{}'\nFound:    '{}'",
                    i + 1, ans_line, out_line
                ));
            }
        }

        // 5. 行数不匹配
        CheckerResult::WrongAnswer(format!(
            "Line number mismatch: Expected {} row, actual {} row",
            answer_lines.len(), output_lines.len()
        ))
    }
}
```

### 示例

#### 示例 1: 完全匹配

```
输出: "Hello\nWorld\n"
答案: "Hello\nWorld"
结果: Accepted
```

#### 示例 2: 行尾空白差异

```
输出: "Hello   \nWorld\t\n"
答案: "Hello\nWorld"
结果: Accepted
```

#### 示例 3: 末尾空行差异

```
输出: "Hello\nWorld\n\n"
答案: "Hello\nWorld"
结果: Accepted
```

#### 示例 4: 内容差异

```
输出: "Hello\nWorld"
答案: "Hello\nEarth"
结果: WrongAnswer("In line 2:\nExpected: 'Earth'\nFound:    'World'")
```

#### 示例 5: 行数差异

```
输出: "Hello\nWorld\nExtra"
答案: "Hello\nWorld"
结果: WrongAnswer("Line number mismatch: Expected 2 row, actual 3 row")
```

### 配置使用

在 `info.toml` 中配置：

```toml
[problem_info]
problem_type = "Standard"
checker_type = "Standard"
```

## 特殊检查器

### 设计原理

特殊检查器通过执行外部程序来检查用户输出。这允许实现复杂的检查逻辑，如浮点数容差检查、部分分数计算等。

### 接口规范

#### 调用方式

```bash
./checker <input_file> <output_file> <answer_file>
```

#### 参数说明

- `<input_file>`: 测试用例输入文件路径
- `<output_file>`: 用户程序输出文件路径
- `<answer_file>`: 期望答案文件路径

#### 返回值规范

| 退出码 | 含义   | stderr 要求              |
|-----|------|------------------------|
| 0   | 答案正确 | 任意内容（通常为空）             |
| 7   | 部分正确 | 格式: `points <分数> <消息>` |
| 其他  | 答案错误 | 错误描述信息                 |

#### 部分分数格式

```
points <分数> <消息>
```

- `<分数>`: 浮点数，范围 0.0 - 1.0，表示得分比例
- `<消息>`: 可选描述信息

### 实现示例

#### 使用 testlib 的 C++ 检查器

```cpp
#include "testlib.h"
#include <cmath>

int main(int argc, char *argv[]) {
    // 注册参数
    registerTestlibCmd(argc, argv);

    // 读取用户输出和答案
    double user_answer = ouf.readDouble();
    double expected_answer = ans.readDouble();

    // 设置容差
    double absolute_tolerance = 1e-6;
    double relative_tolerance = 1e-6;

    // 比较浮点数
    if (doubleCompare(user_answer, expected_answer,
                      absolute_tolerance, relative_tolerance)) {
        // 完全正确
        quitf(_ok, "Correct answer: %.6f", user_answer);
    } else {
        // 计算部分分数
        double diff = std::abs(user_answer - expected_answer);
        double max_diff = std::abs(expected_answer) * relative_tolerance + absolute_tolerance;
        double score = std::max(0.0, 1.0 - diff / max_diff);

        // 输出部分分数
        quitp(score, "Expected %.6f, found %.6f", expected_answer, user_answer);
    }
}
```

#### 简单的 Python 检查器

```python
#!/usr/bin/env python3
import sys
import math

def main():
    if len(sys.argv) != 4:
        print("Usage: checker <input> <output> <answer>", file=sys.stderr)
        sys.exit(1)

    input_file, output_file, answer_file = sys.argv[1:]

    try:
        # 读取答案
        with open(answer_file, 'r') as f:
            expected = float(f.read().strip())

        # 读取用户输出
        with open(output_file, 'r') as f:
            user = float(f.read().strip())

        # 检查容差
        tolerance = 1e-6
        if math.isclose(user, expected, rel_tol=tolerance, abs_tol=tolerance):
            sys.exit(0)  # 正确
        else:
            # 计算部分分数
            diff = abs(user - expected)
            rel_diff = diff / abs(expected) if expected != 0 else diff
            score = max(0.0, 1.0 - rel_diff)

            if score > 0.5:
                # 部分正确
                print(f"points {score:.3f} Close but not exact", file=sys.stderr)
                sys.exit(7)
            else:
                # 错误
                print(f"Expected {expected}, got {user}", file=sys.stderr)
                sys.exit(1)

    except Exception as e:
        print(f"Checker error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
```

### 系统处理逻辑

#### 执行检查器

```rust
let output = tokio::process::Command::new(checker_path)
.arg(input_path.as_ref())
.arg(output_path.as_ref())
.arg(ans_path.as_ref())
.output()
.await?;
```

#### 解析结果

```rust
match output.status.code() {
Some(0) => Ok(CheckerResult::Accepted),
Some(7) => {
// 解析部分分数
let checker_message = String::from_utf8_lossy( & output.stderr).trim();
if let Some(stripped) = checker_message.strip_prefix("points") {
let points_str = stripped.trim().split_whitespace().next().unwrap_or("");
match points_str.parse::< f64 >() {
Ok(points) => Ok(CheckerResult::PartiallyAccepted(points, message)),
Err(_) => Ok(CheckerResult::WrongAnswer("Invalid points format")),
}
} else {
Ok(CheckerResult::WrongAnswer("Missing points message"))
}
}
_ => Ok(CheckerResult::WrongAnswer(checker_message)),
}
```

### 编译和部署

#### 源代码方式

检查器源代码（如 `checker.cpp`）放置在题目目录的 `data/` 子目录中。

**编译命令**:

```bash
g++ /path/to/checker.cpp -o /path/to/checker \
  -O2 -static -std=c++23 -I/path/to/testlib
```

**系统自动编译**: 如果提供的是源代码文件（`.cpp` 扩展名），系统会自动编译。

#### 二进制方式

预编译的检查器二进制文件（如 `checker`）可以直接使用。

**要求**:

1. Linux 可执行文件
2. 具有可执行权限（系统会自动设置）
3. 静态链接或无动态库依赖

#### testlib 库

推荐使用 testlib 库编写检查器，它提供了丰富的辅助函数。

**testlib 函数**:

- `registerTestlibCmd()`: 注册命令行参数
- `ouf.readX()`: 读取用户输出
- `ans.readX()`: 读取答案
- `inf.readX()`: 读取输入（如果需要）
- `quitf(_ok, ...)`: 退出并标记正确
- `quitp(score, ...)`: 退出并给出部分分数
- `quitf(_wa, ...)`: 退出并标记错误

### 配置使用

在 `info.toml` 中配置：

```toml
[problem_info]
problem_type = "Special"
checker_type = "Special"

[custom_modules]
checker_path = "checker.cpp"  # 或 "checker"（二进制文件）
```

## 交互检查器

### 设计原理

交互检查器用于交互式题目，它作为中间进程在用户程序和评测系统之间进行通信。

### 接口规范

#### 调用方式

```bash
./interactor <input_file> <output_file>
```

#### 参数说明

- `<input_file>`: 测试用例输入文件路径
- `<output_file>`: 交互器输出文件路径（用于记录交互过程）

#### 交互协议

1. 交互器从标准输出向用户程序发送数据
2. 用户程序从标准输入读取数据
3. 用户程序处理数据并向标准输出发送响应
4. 交互器从标准输入读取用户程序的响应
5. 重复直到交互结束

#### 返回值

- `0`: 交互成功，答案正确
- 其他: 交互失败或答案错误，stderr 作为错误信息

### 实现示例

#### 使用 testlib 的交互器

```cpp
#include "testlib.h"

int main(int argc, char *argv[]) {
    // 注册交互模式
    registerInteraction(argc, argv);

    // 从输入文件读取测试数据
    int n = inf.readInt();
    int m = inf.readInt();

    // 向用户程序发送初始数据
    cout << n << " " << m << endl;

    // 交互循环
    for (int i = 0; i < m; i++) {
        // 从用户程序读取查询
        int query = ouf.readInt(1, n);  // 读取 1..n 范围内的整数

        // 处理查询（示例：返回平方）
        int response = query * query;

        // 向用户程序发送响应
        cout << response << endl;
    }

    // 读取用户程序的最终答案
    int user_answer = ouf.readInt();
    int correct_answer = n * m;  // 示例正确答案

    if (user_answer == correct_answer) {
        quitf(_ok, "Correct answer: %d", user_answer);
    } else {
        quitf(_wa, "Expected %d, found %d", correct_answer, user_answer);
    }
}
```

#### 简单的 Python 交互器

```python
#!/usr/bin/env python3
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: interactor <input> <output>", file=sys.stderr)
        sys.exit(1)

    input_file, output_file = sys.argv[1:]

    try:
        # 读取输入文件
        with open(input_file, 'r') as f:
            n = int(f.read().strip())

        # 向用户程序发送数据
        print(n, flush=True)

        # 交互过程
        total = 0
        for i in range(n):
            # 从用户程序读取
            line = sys.stdin.readline()
            if not line:
                print("Unexpected EOF from user program", file=sys.stderr)
                sys.exit(1)

            try:
                x = int(line.strip())
            except ValueError:
                print(f"Invalid number: {line.strip()}", file=sys.stderr)
                sys.exit(1)

            # 处理并响应
            response = x * 2
            print(response, flush=True)
            total += response

        # 读取最终答案
        final_answer = sys.stdin.readline()
        if not final_answer:
            print("No final answer from user program", file=sys.stderr)
            sys.exit(1)

        user_total = int(final_answer.strip())

        if user_total == total:
            sys.exit(0)  # 正确
        else:
            print(f"Expected {total}, got {user_total}", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Interactor error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
```

### 系统集成

#### 沙箱配置

对于交互题，沙箱配置中指定交互器路径：

```rust
let interactor = match problem_type {
ProblemType::Interactive => Some({
let path = get_path_by_id_name( & pid, "data/interactor", true).await ?;
chmod_plus_x( & path).await ?;
path
}),
_ => None,
};
```

#### 执行方式

交互器通过 `judger::run()` 函数的第二个参数传入：

```rust
let res = judger::run( & config, interactor).map_err( | e| {
AijError::Judge(
StatusCode::INTERNAL_SERVER_ERROR,
"JUDGER_RUN_FAILED".to_string(),
format ! ("Judger run failed: {}", e),
)
}) ?;
```

#### 结果处理

交互题的结果由交互器决定：

- 交互器正常退出（退出码 0）→ `Accepted`
- 交互器非正常退出 → `WrongAnswer`，使用交互器的 stderr 作为错误信息

### 配置使用

在 `info.toml` 中配置：

```toml
[problem_info]
problem_type = "Interactive"
checker_type = "Interactor"

[custom_modules]
interactor_path = "interactor.cpp"  # 或 "interactor"（二进制文件）
```

## 检查器开发指南

### 最佳实践

#### 1. 错误处理

- 检查所有输入文件的可用性
- 验证输入数据的格式和范围
- 提供清晰的错误信息

#### 2. 性能考虑

- 避免内存泄漏
- 处理大文件时使用流式处理
- 设置合理的超时限制

#### 3. 可移植性

- 使用标准库函数
- 避免平台特定代码

#### 4. 安全性

- 验证输入数据，防止缓冲区溢出
- 避免系统命令注入
- 限制资源使用