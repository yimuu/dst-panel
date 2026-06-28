#!/bin/bash
set -e

# 获取命令行参数
if [ "$#" -gt 0 ]; then
  TAG=$1
else
  DEFAULT_TAG=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
  TAG=$DEFAULT_TAG
fi
IMAGE_NAME=${IMAGE_NAME:-yimuu/dst-panel}

if [ -z "$TAG" ]; then
  echo "usage: ./tools/release/docker-build.sh <tag>" >&2
  echo "or set the package version in Cargo.toml before running without a tag." >&2
  exit 1
fi

# 构建匹配 docker/Dockerfile linux/amd64 平台的 Rust 二进制。
RUST_TARGET=x86_64-unknown-linux-gnu ./tools/release/build-linux.sh

# 构建镜像
docker build --platform linux/amd64 -f docker/Dockerfile -t "$IMAGE_NAME:$TAG" .

# 推送镜像到Docker Hub
docker push "$IMAGE_NAME:$TAG"
