#!/usr/bin/env bash
# Собирает статический Linux-бинарник awgram (musl + rustls) в Docker.
# Использование: ./scripts/build-musl.sh [amd64|arm64|all]   (по умолчанию amd64)
# Флаги сборки (crt-static, relocation-model=static) — в .cargo/config.toml,
# strip — через [profile.release] в Cargo.toml; здесь только оркестрация Docker.
# Результат не зависит от glibc/libssl и запускается на любом Linux своей архитектуры.
set -euo pipefail

cd "$(dirname "$0")/.."

build_one() {
  local arch="$1" platform target
  case "$arch" in
    amd64) platform=linux/amd64 target=x86_64-unknown-linux-musl ;;
    arm64) platform=linux/arm64 target=aarch64-unknown-linux-musl ;;
    *) echo "Неизвестная архитектура: $arch (ожидается amd64|arm64|all)" >&2; exit 1 ;;
  esac

  local cc_var="CC_${target//-/_}"
  local linker_var
  linker_var="CARGO_TARGET_$(echo "$target" | tr '[:lower:]-' '[:upper:]_')_LINKER"

  docker run --rm --platform "$platform" \
    -v "$PWD":/app -w /app \
    -v "awgram-cargo-registry-$arch":/usr/local/cargo/registry \
    rust:1-bookworm bash -c "
      set -e
      apt-get update -qq && apt-get install -y -qq musl-tools >/dev/null
      rustup target add $target >/dev/null
      export $cc_var=musl-gcc
      export $linker_var=musl-gcc
      cargo build --release --target $target
      # сборка идёт от root — возвращаем target/ владельцу репозитория,
      # иначе на Linux-хосте локальный cargo спотыкается о root-owned файлы
      chown -R \"\$(stat -c '%u:%g' /app)\" /app/target
    "

  mkdir -p dist
  cp "target/$target/release/awgram" "dist/awgram-linux-$arch"
  echo "Готово: dist/awgram-linux-$arch"
  file "dist/awgram-linux-$arch"
}

case "${1:-amd64}" in
  all) build_one amd64; build_one arm64 ;;
  *)   build_one "${1:-amd64}" ;;
esac
