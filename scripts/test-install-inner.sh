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

# --- сценарий 2: hardened reinstall поверх root-установки ---
bash /repo/install.sh install --yes --lang en --mode hardened \
  --token T2 --admins 5 --binary-file /tmp/fakebin --no-systemd
id awgram >/dev/null 2>&1 || fail "hardened user"
[ -f /etc/sudoers.d/awgram ] || fail "sudoers file"
visudo -c -f /etc/sudoers.d/awgram >/dev/null || fail "sudoers invalid"
grep -q 'awgram ALL=(root) NOPASSWD: /root/awg/manage_amneziawg.sh' /etc/sudoers.d/awgram || fail "sudoers content"
[ "$(stat -c %a /etc/sudoers.d/awgram)" = "440" ] || fail "sudoers perms"
grep -q '^User=awgram$' /etc/systemd/system/awgram.service || fail "unit User="
grep -q '^sudo_prefix   = "sudo"$' /etc/awgram/config.toml || fail "sudo_prefix hardened"
grep -q '^AWGRAM_TOKEN=T2$' /etc/awgram/env || fail "token updated"
getfacl /root/awg 2>/dev/null | grep -q '^user:awgram:r-x' || fail "acl"
getfacl /root/awg 2>/dev/null | grep -q '^default:user:awgram:r-x' || fail "default acl"
# --yes-переустановка без --mode не должна терять hardened
bash /repo/install.sh install --yes \
  --token T3 --admins 7 --binary-file /tmp/fakebin --no-systemd
grep -q '^MODE=hardened$' /etc/awgram/setup.conf || fail "mode lost on --yes reinstall"
grep -q '^User=awgram$' /etc/systemd/system/awgram.service || fail "unit user lost"
grep -q '^sudo_prefix   = "sudo"$' /etc/awgram/config.toml || fail "sudo_prefix lost"

# --- сценарий 3: update локальным файлом ---
printf '#!/bin/sh\necho v2\n' > /tmp/fakebin2; chmod +x /tmp/fakebin2
/usr/local/bin/awgram-setup update --binary-file /tmp/fakebin2 --yes --no-systemd
[ "$(sha256sum < /usr/local/bin/awgram)" = "$(sha256sum < /tmp/fakebin2)" ] || fail "update binary content"
[ -f /usr/local/bin/awgram.bak ] || fail "update backup"
[ "$(sha256sum < /usr/local/bin/awgram.bak)" = "$(sha256sum < /tmp/fakebin)" ] || fail "backup is previous binary"

echo "OK: all scenarios passed"
