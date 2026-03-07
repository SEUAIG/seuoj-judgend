# 部署指南

本文档介绍如何部署 SEU AIJ Judge-Endpoint 服务，包括本地部署、Docker 部署和生产环境部署。

## 系统要求

### 硬件要求

| 资源 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 2 核 | 4+ 核 |
| 内存 | 2 GB | 8+ GB |
| 存储 | 10 GB | 50+ GB（根据题目数量） |
| 网络 | 100 Mbps | 1 Gbps |

### 软件要求

#### 必需组件
- **操作系统**: Linux（推荐 Ubuntu 20.04+ 或 CentOS 8+）
- **Rust 工具链**: 1.70+（用于从源码构建）
- **Docker**: 20.10+（用于容器化部署）
- **二进制工具链**: 见下表

#### 编程语言支持

系统需要以下二进制工具来支持不同的编程语言：

| 语言 | 必需工具 | 测试版本 | 安装命令（Ubuntu） |
|------|----------|----------|-------------------|
| C | gcc | 9.4+ | `apt install gcc` |
| C++ | g++ | 9.4+ | `apt install g++` |
| Python 3 | python3 | 3.12+ | `apt install python3` |
| Node.js | node | 22+ | `apt install nodejs` |
| Go | go | 1.22+ | `apt install golang` |
| Java | java, javac | 17+ | `apt install openjdk-17-jdk` |

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

### 3. Docker 生产部署

#### 使用 Docker Swarm
```bash
# 初始化 Swarm
docker swarm init

# 部署堆栈
docker stack deploy -c docker-compose.prod.yml aij

# 查看服务
docker service ls
```

#### 使用 Kubernetes

**deployment.yaml**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aij-judgend
spec:
  replicas: 3
  selector:
    matchLabels:
      app: aij-judgend
  template:
    metadata:
      labels:
        app: aij-judgend
    spec:
      containers:
      - name: aij-judgend
        image: aij-judgend:latest
        ports:
        - containerPort: 9090
        env:
        - name: AIJ_LISTEN_ADDR
          value: "0.0.0.0"
        - name: AIJ_LISTEN_PORT
          value: "9090"
        - name: AIJ_MAX_CONCURRENT_REQUESTS
          value: "6"
        - name: AIJ_BACKEND_HOST
          value: "seuoj-backend"
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: problems-volume
          mountPath: /app/assets/problems
        - name: logs-volume
          mountPath: /app/assets/logs
        securityContext:
          capabilities:
            add: ["SYS_ADMIN"]
      volumes:
      - name: problems-volume
        persistentVolumeClaim:
          claimName: aij-problems-pvc
      - name: logs-volume
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: aij-judgend-service
spec:
  selector:
    app: aij-judgend
  ports:
  - port: 9090
    targetPort: 9090
  type: LoadBalancer
```

## 生产环境部署

### 1. 架构设计

#### 单机部署
```
[负载均衡器] → [AIJ Judge-Endpoint] → [后端服务]
                   ↓
              [文件存储]
```

#### 集群部署
```
[负载均衡器] → [AIJ Judge-Endpoint 集群] → [后端服务集群]
                       ↓
                [共享文件存储 (NFS/S3)]
```

### 2. 负载均衡配置

#### Nginx 配置示例
```nginx
upstream aij_backend {
    # 最少连接负载均衡
    least_conn;

    # 后端服务器
    server 10.0.1.1:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.2:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.3:9090 max_fails=3 fail_timeout=30s;

    # 健康检查
    check interval=3000 rise=2 fall=3 timeout=1000;
}

server {
    listen 80;
    server_name judge.example.com;

    # SSL 配置（推荐）
    listen 443 ssl http2;
    ssl_certificate /etc/ssl/certs/judge.example.com.crt;
    ssl_certificate_key /etc/ssl/private/judge.example.com.key;

    location / {
        proxy_pass http://aij_backend;

        # 代理设置
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 超时设置
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 300s;  # 评测可能耗时较长

        # 缓冲区设置
        proxy_buffering off;
        client_max_body_size 100M;  # 与服务配置一致
    }

    # 健康检查端点
    location /health {
        proxy_pass http://aij_backend/health;
        access_log off;
    }
}
```

### 3. 文件存储方案

#### 本地存储（单机）
```bash
# 创建存储目录
mkdir -p /data/aij/{problems,logs,testlib}

# 设置权限
chown -R aij:aij /data/aij
chmod 750 /data/aij

# 更新配置
AIJ_PROBLEMS_DIR=/data/aij/problems
AIJ_LOG_DIR=/data/aij/logs
AIJ_TESTLIB_DIR=/data/aij/testlib
```

#### NFS 共享存储（集群）
```bash
# 服务端配置
# /etc/exports
/data/aij 10.0.0.0/16(rw,sync,no_subtree_check,no_root_squash)

# 客户端挂载
mount -t nfs nfs-server:/data/aij /mnt/aij

# 自动挂载 (/etc/fstab)
nfs-server:/data/aij /mnt/aij nfs defaults 0 0
```

#### 对象存储（S3 兼容）
```rust
// 需要修改代码支持对象存储
// 当前版本仅支持本地文件系统
```

### 4. 监控和告警

#### 健康检查脚本
```bash
#!/bin/bash
# check_aij_health.sh

ENDPOINT="http://localhost:9090/health"
TIMEOUT=5

# 检查服务响应
response=$(curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT $ENDPOINT)

if [ "$response" = "200" ]; then
    echo "OK: AIJ service is healthy"
    exit 0
else
    echo "CRITICAL: AIJ service is unhealthy (HTTP $response)"
    exit 2
fi
```

#### Prometheus 指标

需要添加指标导出（当前版本未实现）：
- 请求计数
- 评测耗时分布
- 并发评测数
- 资源使用情况

#### 日志收集

配置日志转发到集中式日志系统：
```bash
# rsyslog 配置
# /etc/rsyslog.d/60-aij.conf
$ModLoad imfile
$InputFileName /data/aij/logs/aij-*.log
$InputFileTag aij-judgend:
$InputFileStateFile stat-aij-judgend
$InputFileSeverity info
$InputFileFacility local7
$InputRunFileMonitor

local7.* @log-server:514
```

### 5. 备份和恢复

#### 备份脚本
```bash
#!/bin/bash
# backup_aij.sh

BACKUP_DIR="/backup/aij"
DATE=$(date +%Y%m%d_%H%M%S)

# 创建备份目录
mkdir -p $BACKUP_DIR/$DATE

# 备份题目数据
rsync -av --delete /data/aij/problems/ $BACKUP_DIR/$DATE/problems/

# 备份配置文件
cp /opt/aij-judgend/.env $BACKUP_DIR/$DATE/
cp /etc/systemd/system/aij-judgend.service $BACKUP_DIR/$DATE/

# 压缩备份
tar -czf $BACKUP_DIR/aij_backup_$DATE.tar.gz -C $BACKUP_DIR/$DATE .

# 清理旧备份（保留最近30天）
find $BACKUP_DIR -name "aij_backup_*.tar.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR/aij_backup_$DATE.tar.gz"
```

#### 恢复脚本
```bash
#!/bin/bash
# restore_aij.sh

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file>"
    exit 1
fi

# 停止服务
systemctl stop aij-judgend

# 解压备份
tar -xzf $BACKUP_FILE -C /tmp/aij_restore

# 恢复数据
rsync -av --delete /tmp/aij_restore/problems/ /data/aij/problems/

# 恢复配置
cp /tmp/aij_restore/.env /opt/aij-judgend/
cp /tmp/aij_restore/aij-judgend.service /etc/systemd/system/

# 重启服务
systemctl daemon-reload
systemctl start aij-judgend

# 清理临时文件
rm -rf /tmp/aij_restore

echo "Restore completed"
```

## 安全配置

### 1. 网络隔离

```bash
# iptables 规则示例
iptables -A INPUT -p tcp --dport 9090 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 9090 -j DROP
```

### 2. 服务加固

#### 最小权限原则
```bash
# 创建专用用户
useradd -r -s /bin/false aij

# 设置目录权限
chown -R aij:aij /opt/aij-judgend
chmod 750 /opt/aij-judgend
find /opt/aij-judgend -type f -exec chmod 640 {} \;
```

#### 沙箱配置
确保内核支持并正确配置：
```bash
# 检查 seccomp 支持
grep CONFIG_SECCOMP= /boot/config-$(uname -r)

# 检查 cgroups 支持
grep CONFIG_CGROUPS= /boot/config-$(uname -r)
```

### 3. 证书和 TLS

```bash
# 生成自签名证书（测试）
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# 使用 Let's Encrypt（生产）
certbot certonly --nginx -d judge.example.com
```

## 性能调优

### 1. 系统参数优化

```bash
# 增加文件描述符限制
echo "aij soft nofile 65536" >> /etc/security/limits.conf
echo "aij hard nofile 65536" >> /etc/security/limits.conf

# 调整内核参数
echo "net.core.somaxconn = 1024" >> /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 1024" >> /etc/sysctl.conf
echo "vm.overcommit_memory = 1" >> /etc/sysctl.conf
sysctl -p
```

### 2. 服务配置优化

根据负载调整环境变量：
```env
# 并发设置
AIJ_MAX_CONCURRENT_REQUESTS=20  # 根据 CPU 核心数调整

# 资源限制
AIJ_OUTPUT_TRUNCATE_LENGTH=500  # 增加输出截断长度

# 工具链优化
AIJ_TOOLCHAINS=gcc,g++,python3  # 只启用需要的工具
```

### 3. 存储优化

```bash
# 使用 SSD 存储
# 配置适当的 RAID 级别
# 定期清理临时文件

# 清理旧提交（如果保存的话）
find /data/aij/problems/submissions -type d -mtime +7 -exec rm -rf {} \;
```

## 故障排除

### 1. 服务启动失败

**检查步骤**:
```bash
# 查看日志
journalctl -u aij-judgend -f

# 检查端口占用
ss -tlnp | grep :9090

# 检查配置文件
cat /opt/aij-judgend/.env

# 检查二进制工具
which gcc g++ python3 node java javac go
```

### 2. 评测失败

**常见问题**:
- 题目目录不存在
- 测试用例文件缺失
- 检查器编译失败
- 沙箱权限不足

**调试方法**:
```bash
# 启用详细日志
export RUST_LOG=debug
systemctl restart aij-judgend

# 检查题目文件结构
tree /data/aij/problems/1

# 手动测试检查器
cd /data/aij/problems/1/data
./checker 1.in 1.out 1.ans
echo $?
```

### 3. 性能问题

**监控指标**:
```bash
# CPU 使用
top -p $(pgrep aij-judgend)

# 内存使用
pmap -x $(pgrep aij-judgend) | tail -1

# 磁盘 I/O
iotop -p $(pgrep aij-judgend)

# 网络连接
ss -tpn | grep aij-judgend
```

## 升级指南

### 1. 版本升级

```bash
# 停止服务
systemctl stop aij-judgend

# 备份当前版本
cp /opt/aij-judgend/target/release/aij-judgend /opt/aij-judgend/aij-judgend.backup

# 获取新版本
cd /opt/aij-judgend
git pull origin main

# 构建新版本
cargo build --release

# 重启服务
systemctl start aij-judgend

# 验证升级
curl http://localhost:9090/health
```

### 2. 配置迁移

检查配置变更并更新 `.env` 文件：
```bash
# 比较配置模板
diff .env.example .env

# 应用新配置
cp .env.example .env.new
# 手动合并自定义配置
mv .env.new .env
```

### 3. 数据迁移

如果有数据格式变更，需要迁移脚本：
```bash
# 示例迁移脚本
for pid in /data/aij/problems/*; do
    # 迁移旧格式到新格式
    python3 migrate_problem.py $pid
done
```

## 附录

### 环境变量参考

完整环境变量列表见 [配置文档](configuration.md)。

### 依赖版本矩阵

| 组件 | 最低版本 | 推荐版本 | 测试版本 |
|------|----------|----------|----------|
| Rust | 1.70 | 1.75+ | 1.80 |
| Linux 内核 | 4.4 | 5.10+ | 6.1 |
| Docker | 20.10 | 24.0+ | 25.0 |
| gcc/g++ | 9.4 | 11.3+ | 13.2 |
| Python | 3.12 | 3.12 | 3.12 |
| Node.js | 22 | 22 | 22 |
| Go | 1.22 | 1.22 | 1.22 |
| Java | 17 | 17 | 17 |

### 端口说明

| 端口 | 协议 | 用途 | 可配置 |
|------|------|------|--------|
| 9090 | TCP | HTTP API 服务 | 是 (AIJ_LISTEN_PORT) |
| 8080 | TCP | 后端服务通信 | 是 (AIJ_BACKEND_PORT) |

### 文件权限参考

| 路径 | 用户:组 | 权限 | 说明 |
|------|---------|------|------|
| /opt/aij-judgend | aij:aij | 750 | 程序目录 |
| /data/aij/problems | aij:aij | 750 | 题目数据 |
| /data/aij/logs | aij:aij | 750 | 日志目录 |
| /data/aij/testlib | aij:aij | 755 | testlib 库 |
| 检查器二进制文件 | aij:aij | 755 | 需要执行权限 |

### 性能基准

以下为参考性能指标（基于 4 核 8GB 虚拟机）：

| 操作 | 平均耗时 | 峰值耗时 |
|------|----------|----------|
| 编译 C++ 代码（100行） | 0.5s | 2s |
| 执行单个测试用例 | 0.1s | 10s（根据题目） |
| 完整评测（10个测试用例） | 2s | 30s |
| 并发评测（6个并发） | 5s | 60s |
| API 响应时间 | < 10ms | < 100ms |

**注意**: 实际性能受代码复杂度、测试用例大小和系统负载影响。