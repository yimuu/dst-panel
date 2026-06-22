#!/bin/bash
set -e

# 获取命令行参数
TAG=$1
IMAGE_NAME=${IMAGE_NAME:-yimuu/dst-panel}

if [ -z "$TAG" ]; then
  echo "usage: ./docker_build.sh <tag>" >&2
  exit 1
fi

# 构建匹配根 Dockerfile linux/amd64 平台的 Rust 二进制。
RUST_TARGET=x86_64-unknown-linux-gnu ./build_linux.sh

# 构建镜像
docker build --platform linux/amd64 -t "$IMAGE_NAME:$TAG" .

# 推送镜像到Docker Hub
docker push "$IMAGE_NAME:$TAG"
