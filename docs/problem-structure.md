# 题目文件结构

SEU AIJ Judge-Endpoint 使用特定的文件结构来组织题目数据。每个题目都有一个独立的目录，包含元数据、配置文件和测试数据。

## 目录结构

```
assets/problems/
├── {pid}/                    # 题目目录，{pid} 为题目 ID
│   ├── problem.json         # 题目元数据（JSON 格式）
│   ├── info.toml            # 题目配置（TOML 格式）
│   └── data/                # 题目数据目录
│       ├── 1.in             # 测试用例 1 输入
│       ├── 1.ans            # 测试用例 1 答案
│       ├── 2.in             # 测试用例 2 输入
│       ├── 2.ans            # 测试用例 2 答案
│       ├── checker.cpp      # 特殊检查器源代码（可选）
│       ├── checker          # 特殊检查器二进制文件（可选）
│       ├── interactor.cpp   # 交互器源代码（可选）
│       ├── interactor       # 交互器二进制文件（可选）
│       └── std.cpp          # 标准答案程序（可选）
└── ...
```

## 元数据文件 (`problem.json`)

### 格式说明

```json
{
    "pid": "string",           // 题目 ID，必须与目录名一致
    "description": "string",   // 题目描述（Markdown 格式）
    "input": "string",         // 输入格式描述（Markdown 格式）
    "output": "string",        // 输出格式描述（Markdown 格式）
    "hint": "string",          // 提示信息（Markdown 格式）
    "example": [               // 示例列表
        {
            "in": "string",    // 示例输入
            "ans": "string",   // 示例答案
            "description": "string"  // 示例描述（Markdown 格式）
        }
    ]
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `pid` | string | 是 | 题目 ID，必须与目录名一致 |
| `description` | string | 是 | 题目完整描述，支持 Markdown |
| `input` | string | 是 | 输入格式说明，支持 Markdown |
| `output` | string | 是 | 输出格式说明，支持 Markdown |
| `hint` | string | 否 | 解题提示，支持 Markdown |
| `example` | array | 是 | 示例列表，至少包含一个示例 |
| `example[].in` | string | 是 | 示例输入数据 |
| `example[].ans` | string | 是 | 示例答案数据 |
| `example[].description` | string | 否 | 示例说明，支持 Markdown |

### 示例

```json
{
    "pid": "p01",
    "description": "## 两数之和\n\n给定两个整数 a 和 b，计算它们的和。",
    "input": "一行包含两个整数 a 和 b，用空格分隔。\n\n- $1 \\leq a, b \\leq 10^6$",
    "output": "输出一个整数，表示 a 和 b 的和。",
    "hint": "注意整数范围，可能需要使用 64 位整数。",
    "example": [
        {
            "in": "1 2",
            "ans": "3",
            "description": "1 + 2 = 3"
        },
        {
            "in": "100 200",
            "ans": "300",
            "description": "100 + 200 = 300"
        }
    ]
}
```

## 配置文件 (`info.toml`)

### 格式说明

```toml
# 题目基本信息
[problem_info]
problem_type = "Standard"    # 题目类型：Standard, Interactive, Special
checker_type = "Standard"    # 检查器类型：Standard, Special, Interactor
time_limit_ms = 1000         # 时间限制（毫秒），-1 表示无限制
memory_limit_kb = 256000     # 内存限制（KB），-1 表示无限制

# 测试用例配置（可多个）
[[testcases]]
id = 1                       # 测试用例 ID（必须唯一）
in_path = "1.in"             # 输入文件路径（相对于 data/ 目录）
ans_path = "1.ans"           # 答案文件路径（相对于 data/ 目录）
weight = 1.0                 # 权重（用于计算分数）
time_limit_ms = null         # 可选，覆盖全局时间限制
memory_limit_kb = null       # 可选，覆盖全局内存限制

[[testcases]]
id = 2
in_path = "2.in"
ans_path = "2.ans"
weight = 2.0

# 子任务配置（可选）
[[subtasks]]
id = 1                       # 子任务 ID（必须唯一）
cases = [1, 2]               # 包含的测试用例 ID 列表
pre_subtasks = []            # 依赖的子任务 ID 列表
score = 100                  # 子任务总分
type = "min"                 # 评分类型：min（最小值）, sum（求和）

# 自定义模块（可选）
[custom_modules]
checker_path = "checker.cpp"     # 特殊检查器路径（相对于 data/ 目录）
interactor_path = "interactor.cpp" # 交互器路径（相对于 data/ 目录）
```

### 字段说明

#### `[problem_info]` 部分

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `problem_type` | string | 是 | `Standard`（标准题）, `Interactive`（交互题）, `Special`（特殊检查器题） |
| `checker_type` | string | 是 | `Standard`（标准检查器）, `Special`（特殊检查器）, `Interactor`（交互检查器） |
| `time_limit_ms` | integer | 是 | 时间限制（毫秒），-1 表示无限制 |
| `memory_limit_kb` | integer | 是 | 内存限制（KB），-1 表示无限制 |

**类型匹配规则**：
- `problem_type = "Standard"` → `checker_type = "Standard"`
- `problem_type = "Special"` → `checker_type = "Special"`
- `problem_type = "Interactive"` → `checker_type = "Interactor"`

#### `[[testcases]]` 部分

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | integer | 是 | 测试用例 ID，必须唯一 |
| `in_path` | string | 是 | 输入文件路径，相对于 `data/` 目录 |
| `ans_path` | string | 是* | 答案文件路径，相对于 `data/` 目录（交互题不需要） |
| `weight` | float | 是 | 权重，用于计算分数 |
| `time_limit_ms` | integer/null | 否 | 覆盖全局时间限制，null 表示使用全局限制 |
| `memory_limit_kb` | integer/null | 否 | 覆盖全局内存限制，null 表示使用全局限制 |

#### `[[subtasks]]` 部分

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | integer | 是 | 子任务 ID，必须唯一 |
| `cases` | array | 是 | 包含的测试用例 ID 列表 |
| `pre_subtasks` | array | 否 | 依赖的子任务 ID 列表，空数组表示无依赖 |
| `score` | integer | 是 | 子任务总分 |
| `type` | string | 是 | 评分类型：`"min"`（取最小值）或 `"sum"`（求和） |

**子任务依赖**：
- 如果子任务 A 依赖于子任务 B，则 B 必须在 A 之前评测
- 如果依赖的子任务中有错误，当前子任务的所有测试用例将被跳过
- 支持多层依赖关系，但不能有循环依赖

#### `[custom_modules]` 部分

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `checker_path` | string/null | 条件 | 特殊检查器路径，`checker_type = "Special"` 时必须 |
| `interactor_path` | string/null | 条件 | 交互器路径，`checker_type = "Interactor"` 时必须 |

### 示例配置

#### 标准题配置

```toml
[problem_info]
problem_type = "Standard"
checker_type = "Standard"
time_limit_ms = 1000
memory_limit_kb = 256000

[[testcases]]
id = 1
in_path = "1.in"
ans_path = "1.ans"
weight = 1.0

[[testcases]]
id = 2
in_path = "2.in"
ans_path = "2.ans"
weight = 2.0
```

#### 带子任务的题目

```toml
[problem_info]
problem_type = "Standard"
checker_type = "Standard"
time_limit_ms = 2000
memory_limit_kb = 512000

[[testcases]]
id = 1
in_path = "easy/1.in"
ans_path = "easy/1.ans"
weight = 1.0

[[testcases]]
id = 2
in_path = "easy/2.in"
ans_path = "easy/2.ans"
weight = 1.0

[[testcases]]
id = 3
in_path = "hard/1.in"
ans_path = "hard/1.ans"
weight = 2.0

[[subtasks]]
id = 1
cases = [1, 2]
pre_subtasks = []
score = 40
type = "min"

[[subtasks]]
id = 2
cases = [3]
pre_subtasks = [1]
score = 60
type = "min"
```

#### 特殊检查器题

```toml
[problem_info]
problem_type = "Special"
checker_type = "Special"
time_limit_ms = 3000
memory_limit_kb = 1024000

[[testcases]]
id = 1
in_path = "1.in"
ans_path = "1.ans"
weight = 1.0

[custom_modules]
checker_path = "checker.cpp"
```

#### 交互题

```toml
[problem_info]
problem_type = "Interactive"
checker_type = "Interactor"
time_limit_ms = 2000
memory_limit_kb = 256000

[[testcases]]
id = 1
in_path = "1.in"
ans_path = ""  # 交互题不需要答案文件，但字段必须存在
weight = 1.0

[custom_modules]
interactor_path = "interactor.cpp"
```

## 数据文件

### 测试用例文件

- **输入文件** (`{num}.in`): 测试用例的输入数据
- **答案文件** (`{num}.ans`): 测试用例的期望输出数据（交互题不需要）

**要求**：
- 文件使用 UTF-8 编码
- 行尾可以是 `\n`（Unix）或 `\r\n`（Windows）
- 文件大小没有硬性限制，但受系统内存限制

### 特殊检查器

#### 源代码方式

检查器源代码（如 `checker.cpp`）将在评测时自动编译。编译命令：

```bash
g++ /path/to/checker.cpp -o /path/to/checker -O2 -static -std=c++23 -I/path/to/testlib
```

#### 二进制方式

预编译的检查器二进制文件（如 `checker`）可以直接使用，但必须：
1. 是 Linux 可执行文件
2. 具有可执行权限
3. 静态链接或无动态库依赖

#### 检查器接口

特殊检查器必须实现以下接口：

```bash
./checker <input_file> <output_file> <answer_file>
```

**返回值**：
- `0`: 答案正确（Accepted）
- `7`: 部分正确（PartiallyAccepted），需要在 stderr 输出 `points <分数> <消息>`
- 其他: 答案错误（WrongAnswer），stderr 作为错误消息

#### 示例检查器（使用 testlib）

```cpp
#include "testlib.h"
#include <cmath>

int main(int argc, char *argv[]) {
    registerTestlibCmd(argc, argv);

    double pa = ouf.readDouble();
    double ja = ans.readDouble();

    if (doubleCompare(pa, ja, 1e-6)) {
        quitf(_ok, "Correct answer");
    } else {
        // 计算部分分数
        double diff = std::abs(pa - ja);
        double score = std::max(0.0, 1.0 - diff / ja);
        quitp(score, "Expected %.6f, found %.6f", ja, pa);
    }
}
```

### 交互器

#### 源代码方式

交互器源代码（如 `interactor.cpp`）将在评测时自动编译，编译命令与检查器相同。

#### 二进制方式

预编译的交互器二进制文件（如 `interactor`）要求与检查器相同。

#### 交互器接口

交互器必须实现以下接口：

```bash
./interactor <input_file> <output_file>
```

其中：
- `<input_file>`: 测试用例输入文件
- `<output_file>`: 交互器与用户程序交互的输出文件

**交互协议**：
1. 交互器从标准输出向用户程序发送数据
2. 用户程序从标准输入读取数据
3. 用户程序向标准输出发送响应
4. 交互器从标准输入读取响应
5. 重复直到交互结束

**返回值**：
- `0`: 交互成功，答案正确（Accepted）
- 其他: 交互失败或答案错误（WrongAnswer），stderr 作为错误消息

#### 示例交互器（使用 testlib）

```cpp
#include "testlib.h"

int main(int argc, char *argv[]) {
    registerInteraction(argc, argv);

    int n = inf.readInt();  // 从输入文件读取
    cout << n << endl;      // 向用户程序发送

    int sum = 0;
    for (int i = 0; i < n; i++) {
        int x = ouf.readInt();  // 从用户程序读取
        sum += x;
    }

    cout << sum << endl;    // 向用户程序发送最终结果

    int user_sum = ouf.readInt();  // 读取用户程序的答案
    if (user_sum == sum) {
        quitf(_ok, "Correct");
    } else {
        quitf(_wa, "Expected %d, found %d", sum, user_sum);
    }
}
```

## 文件命名规范

### 文件名要求

所有文件名必须符合以下正则表达式：

```
^[a-zA-Z0-9_-]+(/[a-zA-Z0-9_-]+)*(\.[a-zA-Z0-9_-]+)*$
```

**允许的字符**：
- 字母（a-z, A-Z）
- 数字（0-9）
- 下划线（_）
- 连字符（-）
- 点号（.）作为扩展名分隔符
- 斜杠（/）作为路径分隔符

**禁止的字符**：
- 空格
- 特殊字符（@, #, $, %, ^, &, *, 等）
- 中文或其他非 ASCII 字符

### 推荐命名规则

1. **测试用例文件**: `{编号}.in`, `{编号}.ans`
2. **子目录测试用例**: `{子目录}/{编号}.in`
3. **检查器文件**: `checker.cpp`, `checker`
4. **交互器文件**: `interactor.cpp`, `interactor`
5. **标准答案**: `std.cpp`, `solution.py` 等

## 题目创建流程

### 手动创建

1. **创建题目目录**
   ```bash
   mkdir -p assets/problems/p01
   mkdir -p assets/problems/p01/data
   ```

2. **创建元数据文件** (`problem.json`)
   ```bash
   vim assets/problems/p01/problem.json
   ```

3. **创建配置文件** (`info.toml`)
   ```bash
   vim assets/problems/p01/info.toml
   ```

4. **创建测试数据**
   ```bash
   echo "1 2" > assets/problems/p01/data/1.in
   echo "3" > assets/problems/p01/data/1.ans
   ```

5. **创建检查器/交互器**（如果需要）
   ```bash
   vim assets/problems/p01/data/checker.cpp
   ```

### 使用 API 创建

1. **创建题目目录和元数据**
   ```bash
   curl -X PATCH http://localhost:9090/judge/problem/edit \
     -H "Content-Type: application/json" \
     -d '{"pid":"p01","description":"...","input":"...","output":"...","hint":"...","example":[{"in":"1 2","ans":"3","description":""}]}'
   ```

2. **上传题目数据**
   ```bash
   curl -X POST http://localhost:9090/judge/problem/data/p01 \
     -F "file=@data.zip"
   ```

3. **更新题目配置**
   ```bash
   curl -X PUT http://localhost:9090/judge/problem/config/p01 \
     -H "Content-Type: application/json" \
     -d '{"problem_info":{"problem_type":"Standard","checker_type":"Standard","time_limit_ms":1000,"memory_limit_kb":256000},"testcases":[{"id":1,"in_path":"1.in","ans_path":"1.ans","weight":1.0}],"subtasks":[],"custom_modules":{}}'
   ```

## 文件验证

### 自动验证

服务在以下时机验证题目文件：

1. **获取题目信息时**: 验证 `problem.json` 格式
2. **评测时**: 验证 `info.toml` 格式和测试用例文件存在性
3. **上传数据时**: 验证 ZIP 文件内容和文件名

### 手动验证

可以使用以下命令验证题目文件：

```bash
# 验证 JSON 格式
python3 -m json.tool assets/problems/p01/problem.json > /dev/null && echo "JSON valid"

# 验证 TOML 格式
python3 -c "import toml; toml.load('assets/problems/p01/info.toml')" && echo "TOML valid"

# 检查文件是否存在
test -f assets/problems/p01/data/1.in && echo "Input file exists"
test -f assets/problems/p01/data/1.ans && echo "Answer file exists"
```

## 常见问题

### 1. 题目目录不存在

**错误**: `Problem with ID {pid} does not exist`

**解决**: 创建题目目录和必要的文件。

### 2. 配置文件格式错误

**错误**: `INVALID_JSON` 或 `INVALID_TOML`

**解决**: 使用 JSON/TOML 验证工具检查文件格式。

### 3. 测试用例文件不存在

**错误**: `File does not exist: {file_path}`

**解决**: 确保测试用例文件在正确的路径下。

### 4. 检查器/交互器编译失败

**错误**: `Compilation failed`

**解决**: 检查源代码语法和依赖（如 testlib）。

### 5. 文件名无效

**错误**: `Invalid filename`

**解决**: 确保文件名只包含允许的字符，不包含路径遍历（如 `../`）。