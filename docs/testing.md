# 测试说明

本文档详细描述 SEU AIJ Judge-Endpoint 项目的测试策略、测试用例和测试环境。

## 测试策略

### 测试层次

项目采用多层测试策略：

| 测试类型 | 位置 | 工具 | 目的 |
|----------|------|------|------|
| 单元测试 | 模块内部 `#[cfg(test)]` | Rust 内置 | 测试单个函数/方法 |
| 集成测试 | `tests/` 目录 | `axum-test` | 测试 API 接口和模块集成 |
| 功能测试 | 手动/脚本测试 | `curl`, 自定义脚本 | 测试完整功能流程 |
| 性能测试 | 基准测试 | `criterion`, 负载测试工具 | 测试性能和并发能力 |

### 测试覆盖目标

- **代码覆盖率**: >80%
- **关键路径**: 100%
- **错误处理**: 所有错误分支
- **边界条件**: 所有边界情况
- **并发场景**: 并发请求处理

## 测试环境

### 环境要求

#### 必需组件
- Rust 工具链 1.70+
- 所有支持的编程语言工具链
- 测试题目数据 (`assets/problems/1/`)

#### 测试数据
测试需要示例题目数据，位于 `assets/problems/1/`：

```bash
# 确保测试数据存在
test -f assets/problems/1/problem.json || echo "Missing test data"
test -f assets/problems/1/info.toml || echo "Missing test config"
test -f assets/problems/1/data/1.in || echo "Missing test input"
test -f assets/problems/1/data/1.ans || echo "Missing test answer"
```

### 环境设置

#### 开发环境设置脚本

```bash
#!/bin/bash
# scripts/setup_test_env.sh

# 创建测试目录结构
mkdir -p assets/problems/1/data

# 创建示例题目元数据
cat > assets/problems/1/problem.json << 'EOF'
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
EOF

# 创建示例题目配置
cat > assets/problems/1/info.toml << 'EOF'
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

[[subtasks]]
id = 1
cases = [1]
pre_subtasks = []
score = 100
type = "min"
EOF

# 创建测试数据
echo "1 2" > assets/problems/1/data/1.in
echo "3" > assets/problems/1/data/1.ans

echo "Test environment setup complete"
```

## 单元测试

### 测试位置

单元测试位于各个模块的 `#[cfg(test)]` 部分：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        // 测试代码
    }
}
```

### 测试示例

#### 标准检查器测试

```rust
// src/judger/checker/standard.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standard_checker_str() {
        // 完全匹配
        let output = "Hello\nWorld";
        let answer = "Hello\nWorld";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::Accepted
        ));

        // 行尾空白差异
        let output = "Hello   \nWorld\t\n";
        let answer = "Hello\nWorld";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::Accepted
        ));

        // 内容差异
        let output = "Hello\nWorld";
        let answer = "Hello\nEarth";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::WrongAnswer(_)
        ));
    }
}
```

#### 配置文件解析测试

```rust
// src/schema.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        let pid = "1";
        let metadata = ProblemMetadata::from_pid(pid).await;
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.pid, "1");
        assert_eq!(metadata.example.len(), 1);
        assert_eq!(metadata.example[0].ans, "3");
    }

    #[test]
    fn test_problem_config_from_toml() {
        let toml_str = r#"
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
        "#;

        let config = ProblemConfig::from_toml_str(toml_str);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.problem_info.time_limit_ms, 1000);
        assert_eq!(config.testcases.len(), 1);
        assert_eq!(config.testcases[0].id, 1);
    }
}
```

#### 工具函数测试

```rust
// src/judger/utils.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SubtaskConfig;

    #[test]
    fn test_get_topo_order() {
        // 无依赖的子任务
        let subtasks = vec![
            SubtaskConfig {
                id: 1,
                cases: vec![1],
                pre_subtasks: vec![],
                score: 100,
                r#type: "min".to_string(),
            },
            SubtaskConfig {
                id: 2,
                cases: vec![2],
                pre_subtasks: vec![],
                score: 100,
                r#type: "min".to_string(),
            },
        ];

        let order = get_topo_order(&subtasks);
        assert!(order.is_ok());
        let order = order.unwrap();
        assert_eq!(order.len(), 2);
        assert!(order.contains(&1));
        assert!(order.contains(&2));

        // 有依赖的子任务
        let subtasks = vec![
            SubtaskConfig {
                id: 1,
                cases: vec![1],
                pre_subtasks: vec![],
                score: 100,
                r#type: "min".to_string(),
            },
            SubtaskConfig {
                id: 2,
                cases: vec![2],
                pre_subtasks: vec![1],
                score: 100,
                r#type: "min".to_string(),
            },
        ];

        let order = get_topo_order(&subtasks);
        assert!(order.is_ok());
        let order = order.unwrap();
        assert_eq!(order, vec![1, 2]);

        // 循环依赖
        let subtasks = vec![
            SubtaskConfig {
                id: 1,
                cases: vec![1],
                pre_subtasks: vec![2],
                score: 100,
                r#type: "min".to_string(),
            },
            SubtaskConfig {
                id: 2,
                cases: vec![2],
                pre_subtasks: vec![1],
                score: 100,
                r#type: "min".to_string(),
            },
        ];

        let order = get_topo_order(&subtasks);
        assert!(order.is_err());
    }
}
```

### 测试辅助函数

```rust
// 在测试模块中定义辅助函数
#[cfg(test)]
mod tests {
    // 创建临时目录
    fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    // 创建测试配置文件
    fn create_test_config() -> ProblemConfig {
        ProblemConfig {
            problem_info: ProblemInfo {
                problem_type: ProblemType::Standard,
                checker_type: CheckerType::Standard,
                time_limit_ms: 1000,
                memory_limit_kb: 256000,
            },
            testcases: vec![TestCaseConfig {
                id: 1,
                in_path: "1.in".to_string(),
                ans_path: "1.ans".to_string(),
                weight: 1.0,
                time_limit_ms: None,
                memory_limit_kb: None,
            }],
            subtasks: vec![],
            custom_modules: None,
        }
    }

    // 模拟文件系统操作
    async fn mock_file_system() -> Result<()> {
        // 模拟实现
        Ok(())
    }
}
```

## 集成测试

### 测试结构

集成测试位于 `tests/` 目录，每个文件测试一个功能模块：

```
tests/
├── submission.rs          # 评测接口测试
├── get_problem.rs        # 题目获取测试
├── edit_problem.rs       # 题目编辑测试
├── get_problem_config.rs # 配置获取测试
├── put_problem_config.rs # 配置更新测试
├── get_problem_tree.rs   # 文件树测试
├── problem_file.rs       # 文件访问测试
├── upload_testcases.rs   # 数据上传测试
└── utils/               # 测试工具
```

### 测试服务器设置

使用 `axum-test` 创建测试服务器：

```rust
// tests/utils/test_server.rs
use aij_judgend::app;
use axum_test::TestServer;

pub fn create_test_server() -> TestServer {
    let app = app();
    TestServer::new(app).expect("Failed to create test server")
}
```

### API 测试示例

#### 评测接口测试

```rust
// tests/submission.rs
use aij_judgend::app;
use axum_test::{TestServer, TestResponse};
use serde_json::json;

#[tokio::test]
async fn test_judge_submission() {
    let app = app();
    let server = TestServer::new(app).unwrap();

    // 成功请求
    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "test_submission_1",
            "pid": "1",
            "code": "print(1+2)",
            "language": "Python3_12"
        }))
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["code"], 0);
    assert_eq!(body["message"], "Success");

    // 缺少必填字段
    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "test_submission_2",
            "pid": "1"
            // 缺少 code 和 language
        }))
        .await;

    assert_eq!(response.status_code(), 400);

    // 无效的语言类型
    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "test_submission_3",
            "pid": "1",
            "code": "print(1+2)",
            "language": "InvalidLanguage"
        }))
        .await;

    assert_eq!(response.status_code(), 400);

    // 不存在的题目
    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "test_submission_4",
            "pid": "999",
            "code": "print(1+2)",
            "language": "Python3_12"
        }))
        .await;

    assert_eq!(response.status_code(), 404);
}
```

#### 题目管理接口测试

```rust
// tests/get_problem.rs
use aij_judgend::app;
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_get_problem_by_id() {
    let app = app();
    let server = TestServer::new(app).unwrap();

    // 获取存在的题目
    let response = server
        .get("/judge/problem/1")
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["pid"], "1");
    assert!(!body["description"].as_str().unwrap().is_empty());
    assert!(body["example"].is_array());
    assert_eq!(body["example"][0]["in"], "1 2");
    assert_eq!(body["example"][0]["ans"], "3");

    // 获取不存在的题目
    let response = server
        .get("/judge/problem/999")
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_edit_problem() {
    let app = app();
    let server = TestServer::new(app).unwrap();

    let new_problem = json!({
        "pid": "2",
        "description": "新题目",
        "input": "输入格式",
        "output": "输出格式",
        "hint": "提示",
        "example": [{
            "in": "1",
            "ans": "2",
            "description": "示例"
        }]
    });

    // 编辑题目（创建新题目）
    let response = server
        .patch("/judge/problem/edit")
        .json(&new_problem)
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["code"], 0);

    // 验证题目已创建
    let response = server
        .get("/judge/problem/2")
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["pid"], "2");
    assert_eq!(body["description"], "新题目");
}
```

#### 文件上传测试

```rust
// tests/upload_testcases.rs
use aij_judgend::app;
use axum_test::TestServer;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_upload_problem_data() {
    let app = app();
    let server = TestServer::new(app).unwrap();

    // 创建临时 ZIP 文件
    let mut zip_file = NamedTempFile::new().unwrap();

    // 这里需要实际创建 ZIP 文件内容
    // 简化示例：上传空 ZIP
    zip_file.write_all(&[0x50, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00]).unwrap();
    zip_file.flush().unwrap();

    let zip_path = zip_file.path();

    // 上传数据（需要 multipart 支持）
    let response = server
        .post(&format!("/judge/problem/data/{}", "1"))
        .form(&[("file", zip_path.to_str().unwrap())])
        .await;

    // 注意：axum-test 可能需要扩展来支持 multipart
    // 实际测试可能需要使用 reqwest 或其他客户端
}
```

### 测试数据管理

#### 测试数据工厂

```rust
// tests/utils/factories.rs
use aij_judgend::schema::*;
use serde_json::json;

pub struct TestDataFactory;

impl TestDataFactory {
    pub fn create_problem_metadata(pid: &str) -> ProblemMetadata {
        ProblemMetadata {
            pid: pid.to_string(),
            description: format!("题目 {} 描述", pid),
            input: "输入格式".to_string(),
            output: "输出格式".to_string(),
            hint: "提示信息".to_string(),
            example: vec![ProblemExample {
                r#in: "1 2".to_string(),
                ans: "3".to_string(),
                description: "示例".to_string(),
            }],
        }
    }

    pub fn create_problem_config() -> ProblemConfig {
        ProblemConfig {
            problem_info: ProblemInfo {
                problem_type: ProblemType::Standard,
                checker_type: CheckerType::Standard,
                time_limit_ms: 1000,
                memory_limit_kb: 256000,
            },
            testcases: vec![
                TestCaseConfig {
                    id: 1,
                    in_path: "1.in".to_string(),
                    ans_path: "1.ans".to_string(),
                    weight: 1.0,
                    time_limit_ms: None,
                    memory_limit_kb: None,
                },
                TestCaseConfig {
                    id: 2,
                    in_path: "2.in".to_string(),
                    ans_path: "2.ans".to_string(),
                    weight: 2.0,
                    time_limit_ms: Some(2000),
                    memory_limit_kb: None,
                },
            ],
            subtasks: vec![
                SubtaskConfig {
                    id: 1,
                    cases: vec![1],
                    pre_subtasks: vec![],
                    score: 50,
                    r#type: "min".to_string(),
                },
                SubtaskConfig {
                    id: 2,
                    cases: vec![2],
                    pre_subtasks: vec![1],
                    score: 50,
                    r#type: "min".to_string(),
                },
            ],
            custom_modules: None,
        }
    }

    pub fn create_judge_request(submission_id: &str, pid: &str) -> serde_json::Value {
        json!({
            "submissionId": submission_id,
            "pid": pid,
            "code": Self::sample_code(),
            "language": "Python3_12"
        })
    }

    pub fn sample_code() -> String {
        r#"print(sum(map(int, input().split())))"#.to_string()
    }
}
```

#### 测试数据清理

```rust
// tests/utils/cleanup.rs
use std::fs;
use std::path::Path;

pub struct TestCleanup;

impl TestCleanup {
    pub async fn cleanup_problem(pid: &str) -> std::io::Result<()> {
        let problem_dir = format!("assets/problems/{}", pid);
        if Path::new(&problem_dir).exists() {
            fs::remove_dir_all(&problem_dir)?;
        }
        Ok(())
    }

    pub async fn cleanup_submissions() -> std::io::Result<()> {
        let submissions_dir = "assets/problems/submissions";
        if Path::new(submissions_dir).exists() {
            fs::remove_dir_all(submissions_dir)?;
        }
        Ok(())
    }

    pub async fn cleanup_all() -> std::io::Result<()> {
        Self::cleanup_problem("test").await?;
        Self::cleanup_problem("2").await?;
        Self::cleanup_submissions().await?;
        Ok(())
    }
}
```

## 功能测试

### 端到端测试

#### 完整评测流程测试

```rust
// tests/e2e/judging_flow.rs
#[tokio::test]
async fn test_complete_judging_flow() {
    // 1. 准备测试环境
    let server = create_test_server();

    // 2. 创建题目
    let problem_data = TestDataFactory::create_problem_metadata("test_flow");
    let create_response = server
        .patch("/judge/problem/edit")
        .json(&problem_data)
        .await;
    assert_eq!(create_response.status_code(), 200);

    // 3. 上传测试数据
    // ... 上传 ZIP 文件

    // 4. 配置题目
    let config = TestDataFactory::create_problem_config();
    let config_response = server
        .put("/judge/problem/config/test_flow")
        .json(&config)
        .await;
    assert_eq!(config_response.status_code(), 200);

    // 5. 提交评测
    let judge_request = TestDataFactory::create_judge_request(
        "test_flow_submission",
        "test_flow"
    );
    let judge_response = server
        .post("/judge/submission")
        .json(&judge_request)
        .await;
    assert_eq!(judge_response.status_code(), 200);

    // 6. 清理
    TestCleanup::cleanup_problem("test_flow").await.unwrap();
}
```

#### 多语言支持测试

```rust
// tests/e2e/language_support.rs
#[tokio::test]
async fn test_cpp_language() {
    let server = create_test_server();

    let request = json!({
        "submissionId": "cpp_test",
        "pid": "1",
        "code": r#"
#include <iostream>
using namespace std;
int main() {
    int a, b;
    cin >> a >> b;
    cout << a + b << endl;
    return 0;
}"#,
        "language": "Cpp"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_java_language() {
    let server = create_test_server();

    let request = json!({
        "submissionId": "java_test",
        "pid": "1",
        "code": r#"
import java.util.Scanner;
public class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner(System.in);
        int a = sc.nextInt();
        int b = sc.nextInt();
        System.out.println(a + b);
    }
}"#,
        "language": "Java17"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
}
```

### 边界条件测试

#### 资源限制测试

```rust
// tests/edge_cases/resource_limits.rs
#[tokio::test]
async fn test_time_limit_exceeded() {
    // 测试时间超限
    let server = create_test_server();

    // 创建时间限制很短的题目
    let config = json!({
        "problem_info": {
            "problem_type": "Standard",
            "checker_type": "Standard",
            "time_limit_ms": 1,  // 1ms 限制
            "memory_limit_kb": 256000
        },
        "testcases": [{
            "id": 1,
            "in_path": "1.in",
            "ans_path": "1.ans",
            "weight": 1.0
        }],
        "subtasks": []
    });

    // 设置题目配置
    server.put("/judge/problem/config/time_limit_test")
        .json(&config)
        .await;

    // 提交会超时的代码
    let request = json!({
        "submissionId": "timeout_test",
        "pid": "time_limit_test",
        "code": r#"
import time
time.sleep(0.1)  # 100ms，远超限制
print("done")
"#,
        "language": "Python3_12"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_memory_limit_exceeded() {
    // 测试内存超限
    let server = create_test_server();

    // 创建内存限制很小的题目
    let config = json!({
        "problem_info": {
            "problem_type": "Standard",
            "checker_type": "Standard",
            "time_limit_ms": 1000,
            "memory_limit_kb": 1024  // 1MB 限制
        },
        "testcases": [{
            "id": 1,
            "in_path": "1.in",
            "ans_path": "1.ans",
            "weight": 1.0
        }],
        "subtasks": []
    });

    // 设置题目配置
    server.put("/judge/problem/config/memory_limit_test")
        .json(&config)
        .await;

    // 提交会超内存的代码
    let request = json!({
        "submissionId": "memory_test",
        "pid": "memory_limit_test",
        "code": r#"
# 分配大量内存
data = [0] * (1024 * 1024 * 10)  # 10MB 数组
print(len(data))
"#,
        "language": "Python3_12"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
}
```

#### 错误处理测试

```rust
// tests/edge_cases/error_handling.rs
#[tokio::test]
async fn test_compile_error() {
    let server = create_test_server();

    // 提交有语法错误的代码
    let request = json!({
        "submissionId": "compile_error_test",
        "pid": "1",
        "code": r#"
#include <iostream>
int main() {
    // 缺少分号
    std::cout << "Hello"
    return 0;
}
"#,
        "language": "Cpp"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
    // 编译错误会在后台处理并上报
}

#[tokio::test]
async fn test_runtime_error() {
    let server = create_test_server();

    // 提交会导致运行时错误的代码
    let request = json!({
        "submissionId": "runtime_error_test",
        "pid": "1",
        "code": r#"
# 除以零错误
print(1 / 0)
"#,
        "language": "Python3_12"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_large_output() {
    let server = create_test_server();

    // 提交会产生大量输出的代码
    let request = json!({
        "submissionId": "large_output_test",
        "pid": "1",
        "code": r#"
# 生成大量输出
for i in range(10000):
    print(f"Line {i}: {'x' * 100}")
"#,
        "language": "Python3_12"
    });

    let response = server
        .post("/judge/submission")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), 200);
    // 输出应该被截断
}
```

## 性能测试

### 基准测试

```rust
// benches/judging_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use aij_judgend::judger::{judge, SupportedLanguages};
use tokio::runtime::Runtime;

fn bench_judge_simple(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("judge_simple");

    for language in ["Python3_12", "Cpp", "Java17"].iter() {
        group.bench_with_input(
            BenchmarkId::new("language", language),
            language,
            |b, &lang| {
                let lang_enum = match lang {
                    "Python3_12" => SupportedLanguages::Python3_12,
                    "Cpp" => SupportedLanguages::Cpp,
                    "Java17" => SupportedLanguages::Java17,
                    _ => unreachable!(),
                };

                b.to_async(&rt).iter(|| async {
                    judge(
                        "1".to_string(),
                        sample_code(lang),
                        lang_enum,
                        "benchmark".to_string(),
                    ).await
                })
            },
        );
    }

    group.finish();
}

fn sample_code(language: &str) -> String {
    match language {
        "Python3_12" => "print(sum(map(int, input().split())))".to_string(),
        "Cpp" => r#"
#include <iostream>
using namespace std;
int main() {
    int a, b;
    cin >> a >> b;
    cout << a + b << endl;
    return 0;
}
"#.to_string(),
        "Java17" => r#"
import java.util.Scanner;
public class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner(System.in);
        int a = sc.nextInt();
        int b = sc.nextInt();
        System.out.println(a + b);
    }
}
"#.to_string(),
        _ => "".to_string(),
    }
}

criterion_group!(benches, bench_judge_simple);
criterion_main!(benches);
```

### 并发测试

```rust
// tests/performance/concurrent_requests.rs
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_judge_requests() {
    let server = create_test_server();
    let n_requests = 10;

    // 创建多个并发请求
    let tasks: Vec<_> = (0..n_requests)
        .map(|i| {
            let server = server.clone();
            tokio::spawn(async move {
                let request = json!({
                    "submissionId": format!("concurrent_test_{}", i),
                    "pid": "1",
                    "code": "print(1+2)",
                    "language": "Python3_12"
                });

                let response = server
                    .post("/judge/submission")
                    .json(&request)
                    .await;

                response.status_code()
            })
        })
        .collect();

    // 等待所有请求完成
    let results = futures::future::join_all(tasks).await;

    // 验证所有请求都成功
    for result in results {
        let status_code = result.unwrap();
        assert_eq!(status_code, 200);
    }
}

#[tokio::test]
async fn test_semaphore_limit() {
    let server = create_test_server();

    // 设置最大并发数为 2
    std::env::set_var("AIJ_MAX_CONCURRENT_REQUESTS", "2");

    let start_time = std::time::Instant::now();
    let n_requests = 4;

    let tasks: Vec<_> = (0..n_requests)
        .map(|i| {
            let server = server.clone();
            tokio::spawn(async move {
                let request = json!({
                    "submissionId": format!("semaphore_test_{}", i),
                    "pid": "1",
                    "code": "import time; time.sleep(1); print('done')",
                    "language": "Python3_12"
                });

                server.post("/judge/submission")
                    .json(&request)
                    .await
            })
        })
        .collect();

    futures::future::join_all(tasks).await;
    let elapsed = start_time.elapsed();

    // 由于并发限制为2，4个请求应该至少需要2秒
    // 每个请求1秒，2个并发 => 至少2秒
    assert!(elapsed >= Duration::from_secs(2));
}
```

## 测试运行

### 运行所有测试

```bash
# 运行单元测试和集成测试
cargo test --all-features

# 运行特定测试文件
cargo test --test submission

# 运行特定测试函数
cargo test test_judge_submission

# 显示测试输出
cargo test -- --nocapture

# 并行测试
cargo test -- --test-threads=4

# 基准测试
cargo bench
```

### 测试覆盖率

```bash
# 安装覆盖率工具
cargo install cargo-tarpaulin

# 运行覆盖率测试
cargo tarpaulin --ignore-tests --out Html

# 查看覆盖率报告
open tarpaulin-report.html
```

### 持续集成

GitHub Actions 配置示例：

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: clippy, rustfmt

    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y gcc g++ python3 nodejs golang openjdk-17-jdk

    - name: Setup test environment
      run: bash scripts/setup_test_env.sh

    - name: Check format
      run: cargo fmt --check

    - name: Clippy check
      run: cargo clippy -- -D warnings

    - name: Run tests
      run: cargo test --all-features

    - name: Test coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --ignore-tests --out Html --out Lcov
      continue-on-error: true  # 覆盖率可能不完美

    - name: Upload coverage
      uses: codecov/codecov-action@v3
      with:
        files: lcov.info
```

## 测试最佳实践

### 测试设计原则

1. **独立性**: 测试之间不相互依赖
2. **可重复性**: 测试每次运行结果相同
3. **原子性**: 每个测试一个关注点
4. **及时性**: 测试与代码同步更新
5. **可读性**: 测试代码清晰易懂

### 测试命名规范

```rust
// 好的测试名称
#[test]
fn test_judge_submission_success() { ... }

#[test]
fn test_judge_submission_missing_field() { ... }

#[test]
fn test_judge_submission_invalid_language() { ... }

#[test]
fn test_problem_config_parsing_valid() { ... }

#[test]
fn test_problem_config_parsing_invalid_toml() { ... }

// 避免的测试名称
#[test]
fn test1() { ... }  // 不明确

#[test]
fn test_judge() { ... }  // 太泛泛
```

### 测试数据管理

1. **使用工厂函数**创建测试数据
2. **清理测试数据**避免残留
3. **隔离测试环境**避免相互影响
4. **使用临时目录**管理临时文件

### 异步测试

```rust
#[tokio::test]  // 使用 tokio 测试宏
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread")]  // 多线程测试
async fn test_concurrent() {
    // 并发测试代码
}
```

### 模拟和桩

对于外部依赖，使用 trait 和 mock：

```rust
// 定义 trait
pub trait Storage {
    async fn read_file(&self, path: &str) -> Result<String>;
}

// 实际实现
pub struct FileSystemStorage;

impl Storage for FileSystemStorage {
    async fn read_file(&self, path: &str) -> Result<String> {
        tokio::fs::read_to_string(path).await.map_err(...)
    }
}

// Mock 实现
#[cfg(test)]
pub struct MockStorage {
    pub mock_data: HashMap<String, String>,
}

#[cfg(test)]
impl Storage for MockStorage {
    async fn read_file(&self, path: &str) -> Result<String> {
        self.mock_data.get(path)
            .cloned()
            .ok_or_else(|| AijError::FileSystem(...))
    }
}

// 在测试中使用 mock
#[tokio::test]
async fn test_with_mock() {
    let mut mock = MockStorage {
        mock_data: HashMap::new(),
    };
    mock.mock_data.insert("test.txt".to_string(), "content".to_string());

    let result = some_function(&mock).await;
    assert!(result.is_ok());
}
```

## 测试报告

### 测试结果分析

运行测试后，分析：

1. **测试通过率**: 所有测试是否通过
2. **代码覆盖率**: 哪些代码未覆盖
3. **性能指标**: 测试执行时间
4. **资源使用**: 内存和 CPU 使用

### 测试问题跟踪

发现测试问题时：

1. **记录问题**: 创建 Issue 描述问题
2. **最小复现**: 创建最小复现代码
3. **修复验证**: 修复后验证问题解决
4. **回归测试**: 确保不引入新问题

### 测试维护

定期维护测试代码：

1. **更新测试**: 随代码变更更新测试
2. **清理无用测试**: 删除不再需要的测试
3. **优化测试性能**: 优化慢速测试
4. **补充测试用例**: 增加边界条件测试

## 附录

### 测试工具参考

| 工具 | 用途 | 安装 |
|------|------|------|
| `cargo test` | Rust 内置测试框架 | 内置 |
| `axum-test` | Axum 应用测试 | `cargo add axum-test` |
| `criterion` | 基准测试 | `cargo add criterion` |
| `cargo-tarpaulin` | 代码覆盖率 | `cargo install cargo-tarpaulin` |
| `mockall` | 自动生成 mock | `cargo add mockall` |

### 测试命令速查

```bash
# 基本测试
cargo test                      # 所有测试
cargo test --release           # 发布模式测试
cargo test -- --nocapture      # 显示输出

# 特定测试
cargo test test_name           # 特定测试函数
cargo test --test file_name    # 特定测试文件
cargo test module::test_name   # 模块内测试

# 测试选项
cargo test -- --test-threads=1 # 单线程测试
cargo test -- --skip name      # 跳过特定测试
cargo test -- --list           # 列出所有测试

# 覆盖率
cargo tarpaulin --ignore-tests
cargo tarpaulin --out Html

# 基准测试
cargo bench
cargo bench bench_name
```

### 常见测试问题解决

#### 测试数据问题
```bash
# 确保测试数据存在
ls assets/problems/1/

# 重新生成测试数据
bash scripts/setup_test_env.sh
```

#### 权限问题
```bash
# 沙箱需要权限
sudo setcap cap_sys_admin+ep target/debug/aij-judgend

# 或使用 root 运行测试
sudo -E cargo test
```

#### 并发问题
```bash
# 减少并发线程
cargo test -- --test-threads=1
```

#### 超时问题
```bash
# 增加超时时间
cargo test -- --timeout 30
```