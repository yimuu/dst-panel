# Docker 部署

当前 Docker 镜像只支持 Linux amd64。ARM/Mac 镜像暂不维护。

## 构建并推送镜像

在仓库根目录执行：

```bash
# 默认使用 Cargo.toml 中的版本号作为镜像 tag
./tools/release/docker-build.sh

# 或手动指定 tag
./tools/release/docker-build.sh <tag>
```

脚本会先构建匹配 `docker/Dockerfile` 的 Linux amd64 Rust 二进制，再构建并推送 Docker Hub 镜像。默认镜像名是 `yimuu/dst-panel`，可以通过 `IMAGE_NAME` 覆盖：

```bash
IMAGE_NAME=yourname/dst-panel ./tools/release/docker-build.sh
```

## 本地运行

```bash
mkdir -p ~/dstsave

docker run -d \
  --name dst-admin \
  -p 8082:8082/tcp \
  -p 10888:10888/udp \
  -p 10998:10998/udp \
  -p 10999:10999/udp \
  -v ~/dstsave:/data \
  yimuu/dst-panel:latest
```

如果后续会新增多个世界，建议启动时预留 UDP 端口段：

```bash
docker run -d \
  --name dst-admin \
  -p 8082:8082/tcp \
  -p 10888-11020:10888-11020/udp \
  -v ~/dstsave:/data \
  yimuu/dst-panel:latest
```

## 数据卷

容器统一使用 `/data` 作为持久化目录。入口脚本会在首次启动时把默认配置复制到 `/data/config.yml`，并将应用配置改为 `dataDir: "."`。

| 容器内路径 | 用途 |
|-----------|------|
| `/data/config.yml` | 应用运行配置，默认 `dataDir: "."` |
| `/data/dst_config` | DST 安装和集群默认配置 |
| `/data/klei` | Klei 存档目录 |
| `/data/backup` | 存档备份目录 |
| `/data/mod` | MOD 缓存目录 |
| `/data/steamcmd` | SteamCMD 安装目录 |
| `/data/dst-dedicated-server` | 饥荒服务器文件 |
| `/data/dst-db` | SQLite 数据库文件 |
| `/data/password.txt` | 初始管理员账号信息 |
| `/data/first` | 首次登录标记文件 |
| `/data/dst-admin-go.log` | 应用日志文件，保留兼容文件名 |

首次启动时会自动下载 SteamCMD 和 DST Dedicated Server，通常需要等待 10-20 分钟。
