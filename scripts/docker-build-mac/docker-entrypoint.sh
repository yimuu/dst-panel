#!/bin/bash
set -e

# 修正最大文件描述符数，部分 Docker 默认值过高会导致 screen 运行卡顿。
ulimit -Sn 10000

echo "Initializing ARM64 Docker data structure..."

DATA_DIR="/data"
APP_DIR="/app"

# ===== 数据路径 =====
data_steamcmd="${DATA_DIR}/steamcmd"
data_dst_server="${DATA_DIR}/dst-dedicated-server"
data_backup="${DATA_DIR}/backup"
data_klei="${DATA_DIR}/klei"
data_db_file="${DATA_DIR}/dst-db"
data_mod="${DATA_DIR}/mod"
password_file="${DATA_DIR}/password.txt"
config_file="${DATA_DIR}/config.yml"
dst_config_file="${DATA_DIR}/dst_config"

# ===== 基础目录 =====
mkdir -p "$DATA_DIR"
mkdir -p "$data_steamcmd"
mkdir -p "$data_dst_server"
mkdir -p "$data_backup"
mkdir -p "$data_klei"
mkdir -p "$data_mod"

# ===== 运行时配置 =====
if [ ! -f "$config_file" ]; then
  echo "Creating ARM64 Docker runtime config..."
  cp "$APP_DIR/config.yml" "$config_file"
  sed -i 's|^dataDir:.*|dataDir: "."|' "$config_file"
fi

if [ ! -f "$dst_config_file" ]; then
  echo "Creating ARM64 Docker DST config..."
  cp "$APP_DIR/dst_config" "$dst_config_file"
fi

# ===== 静态资源 =====
# 每次启动都刷新镜像内置资源，避免用户升级镜像后继续使用旧版前端。
echo "Refreshing packaged frontend assets..."
mkdir -p "$DATA_DIR/dist"
cp -a "$APP_DIR/dist/." "$DATA_DIR/dist/"

echo "Refreshing packaged static assets..."
mkdir -p "$DATA_DIR/static"
cp -a "$APP_DIR/static/." "$DATA_DIR/static/"

# ===== dst-db 文件（不存在则创建）=====
if [ ! -f "$data_db_file" ]; then
  echo "Creating empty dst-db file..."
  touch "$data_db_file"
fi

# ===== password.txt（不存在则初始化默认账号）=====
if [ ! -f "$password_file" ]; then
  echo "Initializing default admin account..."
  cat > "$password_file" <<EOF
username=admin
password=123456
displayName=admin
photoURL=xxx
EOF
fi

# ===== Klei 默认目录映射 =====
mkdir -p /root/.klei
ln -sf "$data_klei" /root/.klei/DoNotStarveTogether

# ===== x86_64 运行库 =====
# DST 服务端仍是 x86_64 程序，ARM64 容器通过 Box64 运行它。
echo "Ensuring x86_64 runtime libraries are available..."
dpkg --add-architecture amd64
cat > /etc/apt/sources.list.d/amd64.list <<EOF
deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy main universe multiverse restricted
deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy-updates main universe multiverse restricted
deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy-security main universe multiverse restricted
EOF
apt update
apt install -y libc6:amd64 libstdc++6:amd64

# ===== 下载 DST Dedicated Server =====
echo "Installing or validating DST server with DepotDownloader..."
cd /opt/DepotDownloader
./DepotDownloader -app 343050 -os linux -osarch 64 -dir "$data_dst_server" -validate

if [ -f "$data_dst_server/bin64/dontstarve_dedicated_server_nullrenderer_x64" ]; then
  chmod +x "$data_dst_server/bin64/dontstarve_dedicated_server_nullrenderer_x64"
fi

echo "ARM64 DST server ready at $data_dst_server"

cd "$DATA_DIR"
exec "$APP_DIR/dst-admin-rust"
