# --- 第一阶段：编译 Rust 程序 (使用最新稳定版) ---
FROM rust:latest AS builder

# 替换为中国国内镜像源 (Debian bookworm)
RUN sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources

# 配置 Cargo 使用中科大镜像
RUN mkdir -p /usr/local/cargo/ && \
    echo '[source.crates-io]\nreplace-with = "ustc"\n\n[source.ustc]\nregistry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' > /usr/local/cargo/config.toml

# 安装 libseccomp 开发库
RUN apt-get update && apt-get install -y \
    pkg-config \
    libseccomp-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# 编译 Rust
RUN cargo build --release

# --- 第二阶段：全语言运行环境 ---
FROM ubuntu:24.04 AS runtime

# 设置为非交互模式，避免安装过程弹出时区等确认
ENV DEBIAN_FRONTEND=noninteractive

# 替换为中国国内镜像源 (Ubuntu 24.04 noble)
RUN sed -i 's|archive.ubuntu.com|mirrors.aliyun.com|g' /etc/apt/sources.list.d/ubuntu.sources && \
    sed -i 's|security.ubuntu.com|mirrors.aliyun.com|g' /etc/apt/sources.list.d/ubuntu.sources

# 更新源并安装所有运行所需的语言和工具
# Ubuntu 24.04 默认提供：
# - GCC/G++ 13
# - Python 3.12
# - OpenJDK 21
# - Node.js 18+ (或通过官方源安装更高版本)
# - Go 1.22+
RUN apt-get update && apt-get install -y --no-install-recommends \
    libseccomp2 \
    gcc g++ \
    openjdk-21-jdk \
    golang-go \
    python3 \
    nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从编译阶段拷贝 Rust 二进制文件
COPY --from=builder /app/target/release/main ./runner

# 验证各语言版本，确保环境就绪
RUN gcc --version && \
    g++ --version && \
    java -version && \
    go version && \
    python3 --version && \
    node --version

# 启动程序
CMD ["./runner"]
