# 部署指南

本文档介绍如何部署 SEU AIJ Judge-Endpoint 服务，包括本地部署、Docker 部署和生产环境部署。

## 系统要求

### 硬件要求

| 资源  | 最低要求     | 推荐配置           |
|-----|----------|----------------|
| CPU | 2 核      | 4+ 核           |
| 内存  | 2 GB     | 8+ GB          |
| 存储  | 10 GB    | 50+ GB（根据题目数量） |
| 网络  | 100 Mbps | 1 Gbps         |

### 软件要求

#### 必需组件

- **操作系统**: Linux（推荐 Ubuntu 20.04+ 或 CentOS 8+）
- **Rust 工具链**: 1.70+（用于从源码构建）
- **Docker**: 20.10+（用于容器化部署）
- **二进制工具链**: 见下表

#### 编程语言支持

系统需要以下二进制工具来支持不同的编程语言：

| 语言       | 必需工具        | 测试版本  | 安装命令（Ubuntu）                 |
|----------|-------------|-------|------------------------------|
| C        | gcc         | 9.4+  | `apt install gcc`            |
| C++      | g++         | 9.4+  | `apt install g++`            |
| Python 3 | python3     | 3.12+ | `apt install python3`        |
| Node.js  | node        | 22+   | `apt install nodejs`         |
| Go       | go          | 1.22+ | `apt install golang`         |
| Java     | java, javac | 17+   | `apt install openjdk-17-jdk` |

### 安全要求

#### 沙箱支持

系统依赖 Linux 内核特性进行沙箱隔离，需要：

- Linux 内核 4.4+
- seccomp-bpf 支持
- cgroups 支持

#### 权限要求

- 需要 root 权限或 CAP_SYS_ADMIN 能力运行沙箱
- 建议使用非 root 用户运行服务进程
- 需要特定目录的读写权限

## 本地部署

### 1. 环境准备

#### 安装 Rust

```bash
# 安装 Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 配置环境变量
source $HOME/.cargo/env

# 验证安装
rustc --version
cargo --version
```

#### 安装系统依赖

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y \
    gcc g++ python3 nodejs golang openjdk-17-jdk \
    pkg-config libssl-dev build-essential

# CentOS/RHEL
sudo yum install -y \
    gcc gcc-c++ python3 nodejs golang java-17-openjdk-devel \
    openssl-devel make automake gcc-c++ kernel-devel
```

### 2. 获取源代码

```bash
# 克隆仓库
git clone https://github.com/SEUAIG/seuoj-judgend.git
cd seuoj-judgend

# 切换到稳定分支（可选）
git checkout main
```

### 3. 构建项目

```bash
# 调试构建
cargo build

# 发布构建（推荐）
cargo build --release

# 构建产物位置
ls target/release/aij-judgend
```

### 4. 配置环境

#### 创建目录结构

```bash
# 创建必要的目录
mkdir -p assets/{problems,logs,testlib}

# 复制 testlib（如果需要）
# 从 https://github.com/MikeMirzayanov/testlib 获取 testlib
# cp -r testlib/* assets/testlib/
```

#### 创建配置文件

```bash
# 复制示例配置
cp .env.example .env

# 编辑配置
vim .env
```

**.env 示例**:

```env
# 服务器配置
AIJ_LISTEN_ADDR=0.0.0.0
AIJ_LISTEN_PORT=9090
AIJ_MAX_CONCURRENT_REQUESTS=6

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

### 5. 运行服务

#### 直接运行

```bash
# 从源码运行（开发）
cargo run

# 运行构建产物
./target/release/aij-judgend
```

#### 使用系统服务

**systemd 服务文件** (`/etc/systemd/system/aij-judgend.service`):

```ini
[Unit]
Description=SEU AIJ Judge-Endpoint Service
After=network.target

[Service]
Type=simple
User=aij
Group=aij
WorkingDirectory=/opt/aij-judgend
EnvironmentFile=/opt/aij-judgend/.env
ExecStart=/opt/aij-judgend/target/release/aij-judgend
Restart=on-failure
RestartSec=5
LimitNOFILE=65536
LimitNPROC=65536

# 沙箱需要的能力
AmbientCapabilities=CAP_SYS_ADMIN

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/aij-judgend/assets

[Install]
WantedBy=multi-user.target
```

**安装步骤**:

```bash
# 创建专用用户
sudo useradd -r -s /bin/false aij

# 复制文件
sudo cp target/release/aij-judgend /opt/aij-judgend/
sudo cp .env /opt/aij-judgend/
sudo cp -r assets /opt/aij-judgend/

# 设置权限
sudo chown -R aij:aij /opt/aij-judgend
sudo chmod 750 /opt/aij-judgend

# 启用服务
sudo systemctl daemon-reload
sudo systemctl enable aij-judgend
sudo systemctl start aij-judgend
sudo systemctl status aij-judgend
```

### 6. 验证安装

```bash
# 检查服务状态
curl http://localhost:9090/health

# 检查日志
tail -f assets/logs/aij-$(date +%Y-%m-%d).log

# 测试评测（需要题目数据）
curl -X POST http://localhost:9090/judge/submission \
  -H "Content-Type: application/json" \
  -d '{"submissionId":"test1","pid":"1","code":"print(1+2)","language":"Python3_12"}'
```

## Docker 部署

### 1. 构建 Docker 镜像

```bash
# 从源码构建
docker build -t aij-judgend .

# 指定构建参数
docker build \
  --build-arg RUST_VERSION=1.75 \
  --build-arg DEBIAN_VERSION=bullseye \
  -t aij-judgend:latest .
```

### 2. 运行容器

#### 基本运行

```bash
docker run -d \
  --name aij-judgend \
  -p 9090:9090 \
  -v $(pwd)/assets:/app/assets \
  aij-judgend:latest
```

#### 完整配置

```bash
docker run -d \
  --name aij-judgend \
  --restart unless-stopped \
  -p 9090:9090 \
  -v /path/to/problems:/app/assets/problems \
  -v /path/to/logs:/app/assets/logs \
  -v /path/to/testlib:/app/assets/testlib \
  -e AIJ_LISTEN_ADDR=0.0.0.0 \
  -e AIJ_LISTEN_PORT=9090 \
  -e AIJ_MAX_CONCURRENT_REQUESTS=10 \
  -e AIJ_BACKEND_HOST=host.docker.internal \
  -e AIJ_BACKEND_PORT=8080 \
  -e AIJ_SAVE_SUBMISSIONS=false \
  --cap-add=SYS_ADMIN \
  --security-opt apparmor=unconfined \
  aij-judgend:latest
```

#### Docker Compose

**docker-compose.yml**:

```yaml
version: '3.8'

services:
  aij-judgend:
    image: aij-judgend:latest
    build: .
    container_name: aij-judgend
    restart: unless-stopped
    ports:
      - "9090:9090"
    volumes:
      - ./assets/problems:/app/assets/problems
      - ./assets/logs:/app/assets/logs
      - ./assets/testlib:/app/assets/testlib
      - /var/run/docker.sock:/var/run/docker.sock  # 可选，用于监控
    environment:
      - AIJ_LISTEN_ADDR=0.0.0.0
      - AIJ_LISTEN_PORT=9090
      - AIJ_MAX_CONCURRENT_REQUESTS=6
      - AIJ_BACKEND_HOST=backend
      - AIJ_BACKEND_PORT=8080
      - AIJ_SAVE_SUBMISSIONS=false
      - RUST_LOG=info
    cap_add:
      - SYS_ADMIN
    security_opt:
      - apparmor=unconfined
    networks:
      - aij-network

  # 后端服务示例
  backend:
    image: seuoj-backend:latest
    container_name: seuoj-backend
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://user:pass@db:5432/seuoj
    networks:
      - aij-network

  # 数据库示例
  db:
    image: postgres:15
    container_name: seuoj-db
    environment:
      - POSTGRES_DB=seuoj
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres_data:/var/lib/postgresql/data
    networks:
      - aij-network

networks:
  aij-network:
    driver: bridge

volumes:
  postgres_data:
```

**启动命令**:

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f aij-judgend

# 停止服务
docker-compose down
```