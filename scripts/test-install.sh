#!/usr/bin/env bash
# Смоук-тест install.sh в Docker (контейнеры без systemd → --no-systemd).
# Использование: ./scripts/test-install.sh   [TEST_IMAGES="ubuntu:24.04" — переопределить образы]
set -euo pipefail
cd "$(dirname "$0")/.."
read -r -a images <<< "${TEST_IMAGES:-ubuntu:24.04 almalinux:9}"
for img in "${images[@]}"; do
  echo "=== $img ==="
  docker run --rm -v "$PWD":/repo -w /repo "$img" bash /repo/scripts/test-install-inner.sh
done
echo "ALL OK"
