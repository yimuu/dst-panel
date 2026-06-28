# 使用官方的 Ubuntu amd64 基础镜像，匹配 tools/release/build-linux.sh 的默认 Rust target。
FROM --platform=linux/amd64 ubuntu:24.04

LABEL org.opencontainers.image.title="dst-panel"
LABEL org.opencontainers.image.description="Don't Starve Together server panel Rust migration."
LABEL org.opencontainers.image.source="https://github.com/yimuu/dst-panel"

# 更新并安装必要的软件包
RUN dpkg --add-architecture i386 && \
    apt-get update && \
    apt-get install -y \
    curl \
    libcurl3t64-gnutls \
    libcurl3t64-gnutls:i386 \
    lib32gcc-s1 \
    lib32stdc++6 \
    libgcc-s1 \
    libstdc++6 \
    wget \
    ca-certificates \
    screen \
    procps \
    sudo \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 声明数据卷
VOLUME ["/data"]

# 拷贝程序二进制文件
COPY dst-admin-rust /app/dst-admin-rust
RUN chmod 755 /app/dst-admin-rust

COPY docker/entrypoint.sh /app/docker-entrypoint.sh
RUN chmod 755 /app/docker-entrypoint.sh

COPY config.yml /app/config.yml
COPY docker/dst_config /app/dst_config
COPY web-ui/dist /app/dist
COPY static /app/static

# Web 管理面板端口
EXPOSE 8082/tcp
# DST 默认 UDP 端口
EXPOSE 10888/udp
EXPOSE 10998/udp
EXPOSE 10999/udp

# 运行命令
ENTRYPOINT ["./docker-entrypoint.sh"]
