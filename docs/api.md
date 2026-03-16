# API 参考

SEU AIJ Judge-Endpoint 提供以下 HTTP API 接口：

## 基础信息

- **基础 URL**: `http://host:port` (默认: `http://0.0.0.0:9090`)
- **内容类型**: `application/json`
- **响应格式**: 统一使用 JSON 格式

## 健康检查

### GET /health

检查服务是否正常运行。

**响应**:

```json
"OK"
```

**状态码**:

- `200`: 服务正常运行

## 评测接口

### POST /judge/submission

提交代码进行评测。

**请求体**:

```json
{
  "submissionId": "string",
  // 提交 ID
  "pid": "string",
  // 题目 ID
  "code": "string",
  // 源代码
  "language": "string"
  // 编程语言
}
```

**编程语言枚举值**:

- `C`: C 语言
- `Cpp`: C++ (默认版本)
- `Cpp11`: C++11
- `Cpp17`: C++17
- `Cpp20`: C++20
- `Python3_12`: Python 3.12
- `Nodejs22`: Node.js 22
- `Go1_22`: Go 1.22
- `Java17`: Java 17

**响应**:

```json
{
  "code": 0,
  "message": "Success"
}
```

**说明**:

- 该接口是异步的，立即返回接受响应
- 实际评测结果会通过 HTTP PUT 请求上报到后端服务
- 后端地址由环境变量 `AIJ_BACKEND_HOST` 和 `AIJ_BACKEND_PORT` 配置

**错误响应**:

- `400`: 请求体格式错误
- `422`: 题目没有测试用例
- `500`: 服务器内部错误

## 题目管理接口

### GET /judge/problem/{pid}

获取题目元数据。

**路径参数**:

- `pid`: 题目 ID

**响应**:

```json
{
  "pid": "string",
  "description": "string",
  "input": "string",
  "output": "string",
  "hint": "string",
  "example": [
    {
      "in": "string",
      "ans": "string",
      "description": "string"
    }
  ]
}
```

**状态码**:

- `200`: 成功
- `404`: 题目不存在
- `500`: 服务器内部错误

### PATCH /judge/problem/edit

编辑题目元数据。

**请求体**:

```json
{
  "pid": "string",
  "description": "string",
  "input": "string",
  "output": "string",
  "hint": "string",
  "example": [
    {
      "in": "string",
      "ans": "string",
      "description": "string"
    }
  ]
}
```

**响应**:

```json
{
  "code": 0,
  "message": "Success"
}
```

**状态码**:

- `200`: 成功
- `400`: 请求体格式错误
- `500`: 服务器内部错误

### POST /judge/problem/data/{pid}

上传题目数据文件（ZIP 格式）。

**路径参数**:

- `pid`: 题目 ID

**请求头**:

- `Content-Type: multipart/form-data`

**表单字段**:

- `file`: ZIP 文件，包含题目数据文件
- `format`: 文件格式（可选，默认为 "zip"）

**响应**:


```json
{
  "code": 0,
  "message": "Success"
}
```

**说明**:

- ZIP 文件将解压到题目目录的 `data/` 子目录
- 文件名必须匹配正则表达式: `^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)*$`
- 目前只支持 ZIP 格式

**状态码**:

- `200`: 成功
- `400`: 无效的 ZIP 文件或文件名
- `500`: 服务器内部错误

### GET /judge/problem/config/{pid}

获取题目配置。

**路径参数**:

- `pid`: 题目 ID

**响应**:

```json
{
  "problem_info": {
    "problem_type": "Standard",
    "checker_type": "Standard",
    "time_limit_ms": 1000,
    "memory_limit_kb": 256000
  },
  "testcases": [
    {
      "id": 1,
      "in_path": "1.in",
      "ans_path": "1.ans",
      "weight": 1.0,
      "time_limit_ms": null,
      "memory_limit_kb": null
    }
  ],
  "subtasks": [
    {
      "id": 1,
      "cases": [
        1
      ],
      "pre_subtasks": [],
      "score": 100,
      "type": "min"
    }
  ],
  "custom_modules": {
    "checker_path": "checker.cpp",
    "interactor_path": null
  }
}
```

**状态码**:

- `200`: 成功
- `404`: 题目不存在或配置文件不存在
- `500`: 服务器内部错误

### PUT /judge/problem/config/{pid}

更新题目配置。

**路径参数**:

- `pid`: 题目 ID

**请求体**: 与 GET 接口相同的 JSON 结构

**响应**:

```json
{
  "code": 0,
  "message": "Success"
}
```

**状态码**:

- `200`: 成功
- `400`: 请求体格式错误
- `404`: 题目不存在
- `500`: 服务器内部错误

### GET /judge/problem/tree/{pid}

获取题目文件树。

**路径参数**:

- `pid`: 题目 ID

**响应**:

```json
{
  "files": [
    {
      "name": "problem.json",
      "is_dir": false,
      "size": 1234
    },
    {
      "name": "data",
      "is_dir": true,
      "size": 0
    }
  ]
}
```

**状态码**:

- `200`: 成功
- `404`: 题目不存在
- `500`: 服务器内部错误

### GET /judge/problem/file/{pid}/{*filename}

获取题目文件内容。

**路径参数**:

- `pid`: 题目 ID
- `filename`: 文件名（支持路径）

**响应**:

- 文件内容（文本文件）或文件流（二进制文件）

### DELETE /judge/problem/{pid}

删除指定题目及其所有关联文件。

**路径参数**:

- `pid`: 题目 ID

**响应**:

```json
{
  "code": 0,
  "message": "Success"
}
```

**状态码**:

- `204`: 删除成功
- `404`: 题目不存在
- `500`: 服务器内部错误

**说明**:

- 此操作会删除整个题目目录及其所有文件
- 删除操作不可逆，请谨慎使用

### DELETE /judge/problem/file/{pid}/{filename}

删除题目数据文件。

**路径参数**:

- `pid`: 题目 ID
- `filename`: 文件名（支持路径）

**响应**:

```json
{
  "code": 0,
  "message": "Success"
}
```

**状态码**:

- `204`: 删除成功
- `400`: 无效的文件名或文件被题目配置引用
- `404`: 文件不存在
- `500`: 服务器内部错误

**说明**:

- 只能删除 `data/` 目录下的文件
- 如果文件被题目配置（如测试用例或检查器配置）引用，将拒绝删除
- 删除操作不可逆，请谨慎使用

评测完成后，服务会通过以下接口将结果上报到后端：

### PUT /judge/submission/{submissionId} (后端接口)

**URL**: `http://{backend_host}:{backend_port}{backend_prefix}/judge/submission/{submissionId}`

**请求体**:
成功时:

```json
{
  "status": "Success",
  "resultDetail": [
    {
      "id": 1,
      "time": 10,
      "mem": 1024,
      "sys": "Accepted",
      "in": "1 2",
      "ans": "3",
      "out": "3",
      "type": "Accepted",
      "score": 100
    }
  ],
  "subtasks": [
    {
      "id": 1,
      "cases": [
        1
      ],
      "pre_subtasks": [],
      "score": 100,
      "type": "min"
    }
  ]
}
```

编译错误时:

```json
{
  "status": "CompileError",
  "errorDetail": "编译错误信息"
}
```

评测错误时:

```json
{
  "status": "JudgendError",
  "errorDetail": "评测错误信息"
}
```

## 评测结果格式

### 测试用例结果 (`JudgeResultItem`)

```json
{
  "id": 1,
  // 测试用例 ID
  "time": 10,
  // CPU 时间使用（毫秒）
  "mem": 1024,
  // 内存使用（字节）
  "sys": "Accepted",
  // 系统输出信息
  "in": "1 2",
  // 输入内容（截断后）
  "ans": "3",
  // 答案内容（截断后）
  "out": "3",
  // 用户输出（截断后）
  "type": "Accepted",
  // 结果类型
  "score": 100
  // 得分（0-100）
}
```

### 结果类型枚举

- `Accepted`: 答案正确
- `WrongAnswer`: 答案错误
- `TimeLimitExceeded`: 时间超限
- `MemoryLimitExceeded`: 内存超限
- `RuntimeError`: 运行时错误
- `SystemError`: 系统错误
- `PartiallyAccepted`: 部分正确（特殊检查器）
- `Skipped`: 跳过（依赖的子任务错误）

### 子任务结果 (`SubtaskConfig`)

```json
{
  "id": 1,
  // 子任务 ID
  "cases": [
    1,
    2
  ],
  // 包含的测试用例 ID 列表
  "pre_subtasks": [],
  // 依赖的子任务 ID 列表
  "score": 100,
  // 实际得分（评测后更新）
  "type": "min"
  // 评分类型："min" 或 "sum"
}
```

## 错误响应格式

所有错误响应使用统一格式：

```json
{
  "code": -1,
  "message": "错误类型: 详细错误信息"
}
```

### 常见错误代码

- `INVALID_JSON`: JSON 格式错误
- `INVALID_TOML`: TOML 格式错误
- `FILE_NOT_FOUND`: 文件不存在
- `PROBLEM_NOT_FOUND`: 题目不存在
- `NO_TEST_CASES`: 题目没有测试用例
- `COMPILE_ERROR`: 编译错误
- `CIRCULAR_DEPENDENCY`: 子任务循环依赖
- `BINARY_NOT_FOUND`: 二进制工具未找到

## 请求限制

- **请求体大小**: 最大 100 MB（通过中间件限制）
- **并发评测数**: 由环境变量 `AIJ_MAX_CONCURRENT_REQUESTS` 控制（默认 6）
- **输出截断**: 由环境变量 `AIJ_OUTPUT_TRUNCATE_LENGTH` 控制（默认 200 字符）