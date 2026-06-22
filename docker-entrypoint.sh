#!/bin/bash
set -e

# 修正最大文件描述符数
ulimit -Sn 10000

echo "Initializing data structure..."

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
mkdir -p "$data_backup"
mkdir -p "$data_klei"
mkdir -p "$data_mod"

# ===== 运行时配置和静态资源 =====
if [ ! -f "$config_file" ]; then
  echo "Creating Docker runtime config..."
  cp "$APP_DIR/config.yml" "$config_file"
  sed -i 's|^dataDir:.*|dataDir: "."|' "$config_file"
fi

if [ ! -f "$dst_config_file" ]; then
  echo "Creating Docker DST config..."
  cp "$APP_DIR/dst_config" "$dst_config_file"
fi

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

# ===== klei 目录映射 =====
mkdir -p /root/.klei
ln -sf "$data_klei" /root/.klei/DoNotStarveTogether

steam_cmd_path="$data_steamcmd"
steam_dst_server="$data_dst_server"
mkdir -p "$steam_cmd_path"
mkdir -p "$steam_dst_server"

# ============================================================
# 安装 SteamCMD（如果不存在）
# ============================================================

cd "$steam_cmd_path"

retry=1
while [ ! -e "${steam_cmd_path}/steamcmd.sh" ]; do
  if [ $retry -gt 3 ]; then
    echo "Download steamcmd failed after three times"
    exit -2
  fi

  echo "Installing steamcmd, try: ${retry}"
  wget http://media.steampowered.com/installer/steamcmd_linux.tar.gz -P "$steam_cmd_path"
  tar -zxvf "$steam_cmd_path/steamcmd_linux.tar.gz" -C "$steam_cmd_path"
  sleep 3
  ((retry++))
done

# ============================================================
# 安装 DST Dedicated Server（如果不存在）
# ============================================================

retry=1
while [ ! -e "${steam_dst_server}/bin/dontstarve_dedicated_server_nullrenderer" ]; do
  if [ $retry -gt 3 ]; then
    echo "Download DST server failed after three times"
    exit -2
  fi

  echo "Installing DST server, try: ${retry}"
  bash "$steam_cmd_path/steamcmd.sh" \
    +force_install_dir "$steam_dst_server" \
    +login anonymous \
    +app_update 343050 validate \
    +quit

  sleep 3
  ((retry++))
done

echo "SteamCMD ready at $steam_cmd_path"
echo "DST server ready at $steam_dst_server"

cd "$DATA_DIR"
exec "$APP_DIR/dst-admin-rust"
