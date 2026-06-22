# Docker 部署脚本（Mac ARM64）

用于在 Apple Silicon Mac 上构建和运行 DST Admin Rust 的 ARM64 Docker 镜像。DST 服务端仍是 x86_64 程序，镜像通过 Box64 运行游戏服务端，通过 ARM64 版本 DepotDownloader 下载游戏文件。

## 目录内容

- `Dockerfile`：ARM64 镜像构建文件（Ubuntu 22.04）
- `docker-entrypoint.sh`：容器启动脚本，会初始化 `/data` 数据卷
- `docker_dst_config`：ARM64 Docker 默认 DST 配置
- `dst-mac-arm64-env-install.md`：非 Docker 手动安装说明

## 构建镜像

先在仓库根目录生成 ARM64 Linux Rust 二进制：

```bash
rustup target add aarch64-unknown-linux-gnu
RUST_TARGET=aarch64-unknown-linux-gnu ./build_linux.sh
```

然后继续在仓库根目录构建镜像。不要切换到 `scripts/docker-build-mac` 目录，否则 `dist`、`static` 和二进制文件不会进入 Docker build context。

```bash
docker build --platform linux/arm64 -f scripts/docker-build-mac/Dockerfile -t dst-admin-rust-arm64:latest .
```

## 运行容器

```bash
mkdir -p ~/dstsave

docker run -d \
  --name dst-admin-arm64 \
  --platform linux/arm64 \
  -p 8082:8082 \
  -p 10888:10888/udp \
  -p 10998:10998/udp \
  -p 10999:10999/udp \
  -v ~/dstsave:/data \
  dst-admin-rust-arm64:latest
```

启动后访问 `http://localhost:8082`。

## 数据卷

所有运行时数据统一保存在 `/data`，入口脚本会创建默认配置、数据库、账号文件、存档目录、备份目录和游戏服务端目录。

| 容器内路径 | 用途 |
|-----------|------|
| `/data/config.yml` | 面板运行配置，默认 `dataDir: "."` |
| `/data/dst_config` | DST 安装和集群配置 |
| `/data/klei` | Klei 存档目录 |
| `/data/backup` | 存档备份目录 |
| `/data/mod` | MOD 缓存目录 |
| `/data/steamcmd` | 兼容配置项，ARM64 镜像实际使用 DepotDownloader |
| `/data/dst-dedicated-server` | 饥荒服务端文件 |
| `/data/dst-db` | SQLite 数据库文件 |
| `/data/password.txt` | 初始管理员账号文件 |
| `/data/first` | 首次登录标记文件 |
| `/data/dst-admin-go.log` | 应用日志文件，保留兼容文件名 |

## Docker Compose 示例

```yaml
version: "3.8"

services:
  dst-admin-arm64:
    image: dst-admin-rust-arm64:latest
    container_name: dst-admin-arm64
    platform: linux/arm64
    restart: unless-stopped
    ports:
      - "8082:8082"
      - "10888:10888/udp"
      - "10998:10998/udp"
      - "10999:10999/udp"
    volumes:
      - ${PWD}/dstsave:/data
    environment:
      - TZ=Asia/Shanghai
```

## 日志和排查

查看容器日志：

```bash
docker logs -f dst-admin-arm64
```

查看应用日志：

```bash
docker exec -it dst-admin-arm64 cat /data/dst-admin-go.log
```

如果游戏服务端下载失败，检查容器日志中的 DepotDownloader 输出。ARM64 镜像每次启动都会校验 `/data/dst-dedicated-server`，缺失文件会自动补齐。
