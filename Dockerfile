# 使用官方的 Ubuntu amd64 基础镜像，匹配 build_linux.sh 的默认 Rust target。
FROM --platform=linux/amd64 ubuntu:20.04

LABEL maintainer="hujinbo23 jinbohu23@outlook.com"
LABEL description="DoNotStarveTogehter server panel Rust migration. github: https://github.com/hujinbo23/dst-admin-go"

# 更新并安装必要的软件包
RUN dpkg --add-architecture i386 && \
    apt-get update && \
    apt-get install -y \
    curl \
    libcurl4-gnutls-dev:i386 \
    lib32gcc1 \
    lib32stdc++6 \
    libcurl4-gnutls-dev \
    libgcc1 \
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

COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod 755 /app/docker-entrypoint.sh

COPY config.yml /app/config.yml
COPY docker_dst_config /app/dst_config
COPY dist /app/dist
COPY static /app/static

# 内嵌源配置信息
# 控制面板访问的端口
EXPOSE 8082/tcp
# 饥荒世界通信的端口
EXPOSE 10888/udp
# 饥荒洞穴世界的端口
EXPOSE 10998/udp
# 饥荒森林世界的端口
EXPOSE 10999/udp

# 运行命令
ENTRYPOINT ["./docker-entrypoint.sh"]
