# SEU AIJ Judge-Endpoint 文档

## 概述

SEU AIJ Judge-Endpoint 是一个基于 Rust 的在线评测系统服务端，负责接收代码提交、在沙箱环境中执行评测，并返回评测结果。它通过
HTTP API 提供评测服务，支持多种编程语言和题目类型。

## 目录

- [项目架构](architecture.md) - 系统架构和模块设计
- [API 参考](api.md) - HTTP API 接口文档
- [配置文件](configuration.md) - 环境变量和配置文件说明
- [题目文件结构](problem-structure.md) - 题目文件的组织方式
- [评测流程](judging-process.md) - 代码评测的详细流程
- [检查器系统](checkers.md) - 标准检查器和特殊检查器
- [部署指南](deployment.md) - 如何部署和运行服务
- [开发指南](development.md) - 开发相关信息和贡献指南
- [测试说明](testing.md) - 测试策略和测试用例

## 快速开始

### 构建项目

```bash
cargo build --release
```

### 运行服务

```bash
cargo run
```

### 使用 Docker

```bash
docker build -t aij-judgend .
docker run -p 9090:9090 -v $(pwd)/assets:/app/assets aij-judgend
```

## 核心功能

- **多语言支持**: C, C++ (Cpp11, Cpp17, Cpp20), Python 3.12, Node.js 22, Go 1.22, Java 17
- **题目类型**: 标准题、交互题、特殊检查器题
- **检查器类型**: 标准检查器、特殊检查器、交互检查器
- **子任务系统**: 支持子任务依赖关系和评分策略
- **并发控制**: 通过信号量限制并发评测数量
- **沙箱安全**: 使用 `judger` crate 提供进程隔离和资源限制
- **详细日志**: 使用 `tracing` 进行结构化日志记录