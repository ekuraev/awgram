#!/usr/bin/env bash
# Собирает статический Linux amd64 бинарник awg-bot (musl + rustls).
# Результат не зависит от glibc/libssl и запускается на любом Linux x86_64.
# Требуется работающий Docker (сборка идёт в контейнере linux/amd64).
set -euo pipefail

cd "$(dirname "$0")/.."

docker run --rm --platform linux/amd64 \
  -v "$PWD":/app -w /app \
  -v awgbot-cargo-registry:/usr/local/cargo/registry \
  rust:1-bookworm bash -c '
    set -e
    apt-get update -qq && apt-get install -y -qq musl-tools >/dev/null
    rustup target add x86_64-unknown-linux-musl >/dev/null
    export CC_x86_64_unknown_linux_musl=musl-gcc
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
    # crt-static + relocation-model=static → классический статический ET_EXEC
    # без PT_INTERP (иначе musl-загрузчик отсутствует на glibc-хостах).
    export RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static"
    cargo build --release --target x86_64-unknown-linux-musl
    strip target/x86_64-unknown-linux-musl/release/awg-bot
  '

mkdir -p dist
cp target/x86_64-unknown-linux-musl/release/awg-bot dist/awg-bot-linux-amd64
echo "Готово: dist/awg-bot-linux-amd64"
file dist/awg-bot-linux-amd64
