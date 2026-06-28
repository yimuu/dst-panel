# Deployment/部署

注意目录必须要有读写权限

### 脚本一键部署

请加 QQ 群获取

### 二进制部署

请下载最新的 release 版本

[部署教程](https://blog.csdn.net/Dig_hoof/article/details/131296762)

[视频教程](https://www.bilibili.com/read/cv25125509)

### docker 部署

**第一次启动时会自动下载 steamcmd 和饥荒服务器，请耐心等待 10-20 分钟，你也可以使用挂载路径避免下载**

自己映射对应的端口

```
rustup target add x86_64-unknown-linux-gnu
./tools/release/build-linux.sh
docker build --platform linux/amd64 -f docker/Dockerfile -t dst-panel:local .
docker run --name dst -d \
  -p 8082:8082 \
  -p 10999:10999/udp \
  -p 10998:10998/udp \
  -p 10888:10888/udp \
  -v /root/dstsave:/data \
  dst-panel:local
```

**路径参考**

```
+ 容器统一持久化路径: /data
+ 容器存档启动路径: /data/klei
+ 容器存档备份路径: /data/backup
+ 容器存档模组路径: /data/mod
+ 容器数据库路径: /data/dst-db 这是一个文件
+ 容器服务日志路径: /data/dst-admin-go.log
+ 容器启动饥荒路径: /data/dst-dedicated-server
+ 容器启steamcmd：/data/steamcmd
```

#### 旧版本升级说明

1.2.5 及其之前版本的 Docker 挂载方式已归档到对应版本标签；当前 Rust 镜像统一使用 `/data` 持久化路径。
