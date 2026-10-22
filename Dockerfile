# --- 第一阶段：编译 Rust 程序 (使用最新稳定版) ---
FROM rust:1.98.1 AS builder

ENV CARGO_BUILD_JOBS=8

# 替换为中国国内镜像源 (Debian bookworm)
RUN sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources

# 配置 Cargo 使用中科大镜像
RUN mkdir -p /usr/local/cargo/ && \
    echo '[source.crates-io]\nreplace-with = "rsproxy"\n\n[source.rsproxy]\nregistry = "sparse+https://rsproxy.cn/index/"' > /usr/local/cargo/config.toml

# 安装 libseccomp 开发库
RUN apt-get update && apt-get install -y \
    pkg-config \
    libseccomp-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 优化：先只复制依赖清单，构建空壳项目以缓存依赖
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo 'fn main() {}' > src/bin/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release --locked && \
    rm -rf src

# 复制真正的源码并编译（依赖层已缓存）
COPY . .
RUN find src -type f -exec touch {} + && cargo build --release --locked

# Fixed security releases for the languages exposed by the judge.
FROM node:24.22.1-bookworm-slim AS node-toolchain
FROM golang:1.26.10-bookworm AS go-toolchain

# --- 第二阶段：全语言运行环境 ---
FROM ubuntu:24.04 AS runtime

# 设置为非交互模式，避免安装过程弹出时区等确认
ENV DEBIAN_FRONTEND=noninteractive

# 替换为中国国内镜像源 (Ubuntu 24.04 noble)
RUN sed -i 's|archive.ubuntu.com|mirrors.aliyun.com|g' /etc/apt/sources.list.d/ubuntu.sources && \
    sed -i 's|security.ubuntu.com|mirrors.aliyun.com|g' /etc/apt/sources.list.d/ubuntu.sources

# 更新源并安装所有运行所需的语言和工具
# - GCC/G++ 13
# - Python 3.12
# - OpenJDK 21
# - Go 1.26.10
# - Node.js 24.22.1
RUN apt-get update && apt-get install -y --no-install-recommends \
    libseccomp2 \
    gcc g++ \
    openjdk-21-jdk \
    ca-certificates \
    libatomic1 \
    python3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=node-toolchain /usr/local/bin/node /usr/local/bin/node
COPY --from=go-toolchain /usr/local/go /usr/local/go
ENV PATH="/usr/local/go/bin:${PATH}"

# Warm the standard packages used by submissions before the compile timer runs.
RUN printf 'package main\nimport ("fmt"; "os")\nfunc main() { fmt.Fprintln(os.Stdout, "ready") }\n' > /tmp/oj-go-warmup.go && \
    GOMAXPROCS=2 go build -p=1 -o /tmp/oj-go-warmup /tmp/oj-go-warmup.go && \
    rm /tmp/oj-go-warmup.go /tmp/oj-go-warmup

# 从编译阶段拷贝 Rust 二进制文件
COPY --from=builder /app/target/release/main ./runner
COPY --from=builder /app/assets/testlib ./assets/testlib

# 验证各语言版本，确保环境就绪
RUN gcc --version && \
    g++ --version && \
    java -version && \
    go version && \
    dpkg-query -W python3 && \
    node --version

# 启动程序
CMD ["./runner"]
