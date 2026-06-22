# dst-admin-rust
> 饥荒联机版管理后台
> 
> 预览 https://carrot-hu23.github.io/dst-admin-go-preview/

[English](README-EN.md)/[中文](README.md)

**新面板 [泰拉瑞亚面板](https://github.com/carrot-hu23/terraria-panel-app) 支持window,linux 一键启动，内置 1449 版本**

## 项目简介

**现已支持 Windows 和 Linux 平台**
> 注意：Windows Server 低版本系统请使用 1.2.8 之前的版本，高版本系统使用最新版本

DST Admin Rust 是一个使用 Rust 2024 迁移实现的《饥荒联机版》服务器管理面板，目标二进制：`dst-admin-rust`。它保持原面板的部署方式和 API 兼容面，具有以下特点：

- 🚀 **部署简单**：单个可执行文件，无需复杂配置，开箱即用
- 💾 **资源占用低**：基于 Rust 开发，内存占用小，运行高效
- 🎨 **界面美观**：现代化的 Web 界面，操作直观友好
- ⚙️ **功能完善**：
  - 可视化配置游戏房间和世界参数
  - 在线管理和配置 Mod（模组）
  - 支持多个集群（Cluster）和世界的统一管理
  - 游戏存档备份与快照恢复
  - 玩家管理（白名单、黑名单、管理员）
  - 实时日志查看和游戏控制台
  - 游戏服务器自动更新检测

## 部署
注意目录必须要有读写权限。

点击查看 [部署文档](https://carrot-hu23.github.io/dst-admin-go-docs/)

## 预览

![首页效果](docs/image/dashboard.png)
![首页效果](docs/image/panel.png)
![首页效果](docs/image/toomanyitemplus.png)
![首页效果](docs/image/player.png)
![房间效果](docs/image/home.png)
![世界效果](docs/image/level.png)
![世界效果](docs/image/selectormod.png)
![模组效果](docs/image/mod1.png)
![模组效果](docs/image/mod3.png)
![模组效果](docs/image/mod2.png)
![日志效果](docs/image/playerlog.png)
![大厅效果](docs/image/lobby.png)

## 运行

**修改config.yml**
```yaml
#绑定地址
bindAddress: ""
#启动端口
port: 8082
#数据目录前缀
dataDir: "./data"
# wincli端口
dstCliPort: 8102
#数据库
database: dst-db
```

运行
```bash
cargo run --bin dst-admin-rust
```

## 打包

### Linux 打包

```bash
./build_linux.sh
# 输出: dst-admin-rust (Linux amd64 二进制文件)
```

跨平台打 Linux 包时先安装目标并准备 linker：

```bash
rustup target add x86_64-unknown-linux-gnu
LINUX_LINKER=x86_64-linux-gnu-gcc ./build_linux.sh
```

### Windows 打包

```bash
./build_window.sh
# 输出: dst-admin-rust.exe (Windows amd64 二进制文件)
```

Windows GNU 打包需要目标和 MinGW linker：

```bash
rustup target add x86_64-pc-windows-gnu
x86_64-w64-mingw32-gcc --version
./build_window.sh
```

### 直接构建当前平台二进制

```bash
cargo build --release --bin dst-admin-rust
```

## QQ 群
![QQ 群](docs/image/饥荒开服面板交流issue群聊二维码.png)
