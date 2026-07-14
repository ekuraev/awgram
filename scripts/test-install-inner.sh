#!/usr/bin/env bash
# Выполняется в контейнере от root. Задачи 5-8 добавляют сценарии перед финальным echo.
set -euo pipefail
fail() { echo "FAIL: $*" >&2; exit 1; }

mkdir -p /root/awg
printf '#!/bin/sh\necho ok\n' > /root/awg/manage_amneziawg.sh
chmod +x /root/awg/manage_amneziawg.sh
printf '#!/bin/sh\necho awgram-stub\n' > /tmp/fakebin; chmod +x /tmp/fakebin

# --- сценарий 1: root install (без сети GitHub, без systemd) ---
bash /repo/install.sh install --yes --lang en --mode root \
  --token TESTTOKEN --admins 1,2 --binary-file /tmp/fakebin --no-systemd
[ -x /usr/local/bin/awgram ] || fail "binary missing"
# cmp/diff отсутствуют в минимальном almalinux:9 — сверяем содержимое через sha256sum (есть в обоих образах)
[ "$(sha256sum < /usr/local/bin/awgram)" = "$(sha256sum < /tmp/fakebin)" ] || fail "binary content"
grep -q '^admin_ids     = \[1, 2\]$' /etc/awgram/config.toml || fail "admins in config"
grep -q '^AWGRAM_TOKEN=TESTTOKEN$' /etc/awgram/env || fail "token in env"
[ "$(stat -c %a /etc/awgram/env)" = "600" ] || fail "env perms"
grep -q '^sudo_prefix   = ""$' /etc/awgram/config.toml || fail "sudo_prefix root"
grep -q '^manage_script = "/root/awg/manage_amneziawg.sh"$' /etc/awgram/config.toml || fail "manage_script"
grep -q '^clients_dir   = "/root/awg"$' /etc/awgram/config.toml || fail "clients_dir default"
[ -x /usr/local/bin/awgram-setup ] || fail "awgram-setup missing"
grep -q '^LANG=en$' /etc/awgram/setup.conf || fail "lang persisted"
grep -q '^MODE=root$' /etc/awgram/setup.conf || fail "mode persisted"
grep -q '^VERSION=local$' /etc/awgram/setup.conf || fail "version persisted"
[ -f /etc/systemd/system/awgram.service ] || fail "unit file"
grep -q '^User=' /etc/systemd/system/awgram.service && fail "root unit must not have User="

echo "OK: all scenarios passed"
