#!/bin/bash
# dst-go.sh

APP_NAME=dst-admin-rust
ARCHIVE_NAME=dst-admin-rust.tgz
RELEASE_URL="https://github.com/yimuu/dst-panel/releases/download/2.0.0.beta/${ARCHIVE_NAME}"

# 下载并解压 Rust 服务端二进制
download() {
  if command -v wget > /dev/null
  then
    # 执行wget命令
    echo "Downloading ${APP_NAME}..."
    wget "$RELEASE_URL"
    tar -xvf "$ARCHIVE_NAME"
    cd "$APP_NAME"
    chmod +x "$APP_NAME"
  else
    echo "wget command not found."
  fi

}

# 检查 Rust 服务端进程是否运行
check_status() {
  if pgrep "$APP_NAME" > /dev/null
  then
    echo "${APP_NAME} is running."
  else
    echo "${APP_NAME} is not running."
  fi
}

# 启动 Rust 服务端进程
start() {
  echo "Starting ${APP_NAME}..."
  nohup "./${APP_NAME}" > /dev/null 2>&1 &
}

# 关闭 Rust 服务端进程
stop() {
  echo "Stopping ${APP_NAME}..."
  pkill "$APP_NAME"
}

# 显示菜单
# 显示菜单
menu() {
  echo "Please select an option:"
  echo "0. Download ${APP_NAME}"
  echo "1. Check status"
  echo "2. Start"
  echo "3. Stop"
  read option
  case $option in
    0) download ;;
    1) check_status ;;
    2) start ;;
    3) stop ;;
    *) echo "Invalid option. Please try again." ;;
  esac
}

menu
