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
acl_out="$(getfacl /root/awg 2>/dev/null)"
grep -q '^user:awgram:r-x' <<<"$acl_out" || fail "acl"
grep -q '^default:user:awgram:r-x' <<<"$acl_out" || fail "default acl"
# --- Critical 1: config/state доступны User=awgram в hardened ---
[ "$(stat -c %U:%G /etc/awgram/config.toml)" = "root:awgram" ] || fail "config.toml owner"
su -s /bin/sh awgram -c 'cat /etc/awgram/config.toml' >/dev/null || fail "config unreadable by awgram"
su -s /bin/sh awgram -c 'touch /var/lib/awgram/state.json' || fail "state dir unwritable"
grep -q '^state_file = "/var/lib/awgram/state.json"$' /etc/awgram/config.toml || fail "state_file hardened path"
# --yes-переустановка без --mode не должна терять hardened
bash /repo/install.sh install --yes \
  --token T3 --admins 7 --binary-file /tmp/fakebin --no-systemd
grep -q '^MODE=hardened$' /etc/awgram/setup.conf || fail "mode lost on --yes reinstall"
grep -q '^User=awgram$' /etc/systemd/system/awgram.service || fail "unit user lost"
grep -q '^sudo_prefix   = "sudo"$' /etc/awgram/config.toml || fail "sudo_prefix lost"

# --- сценарий 2b: --yes-переустановка БЕЗ --admins берёт admin_ids из config.toml ---
echo '# canary-edit' >> /etc/awgram/config.toml
bash /repo/install.sh install --yes --binary-file /tmp/fakebin --no-systemd \
  || fail "--yes reinstall without --admins must succeed"
grep -q '^admin_ids     = \[7\]$' /etc/awgram/config.toml || fail "admins not preserved on reinstall"
# --- сценарий 2c: переустановка бэкапит config.toml (ручные правки не теряются молча) ---
[ -f /etc/awgram/config.toml.bak ] || fail "no config backup on reinstall"
grep -q '^# canary-edit$' /etc/awgram/config.toml.bak || fail "config backup lacks previous content"

# --- сценарий 2d: валидация manage_script для sudoers (hardened) ---
if bash /repo/install.sh install --yes --script-path 'awg/relative.sh' \
     --binary-file /tmp/fakebin --no-systemd >/dev/null 2>&1; then
  fail "relative script path must be rejected"
fi
if bash /repo/install.sh install --yes --script-path '/root/awg/my script.sh' \
     --binary-file /tmp/fakebin --no-systemd >/dev/null 2>&1; then
  fail "script path with spaces must be rejected in hardened"
fi
printf '#!/bin/sh\necho ww\n' > /root/awg/worldwritable.sh; chmod 777 /root/awg/worldwritable.sh
ww_out="$(bash /repo/install.sh install --yes --script-path /root/awg/worldwritable.sh \
  --binary-file /tmp/fakebin --no-systemd 2>&1)" || fail "install with world-writable script must still succeed"
grep -qi 'writable' <<<"$ww_out" || fail "no warning about world-writable manage script"
# восстанавливаем канонический путь для следующих сценариев
bash /repo/install.sh install --yes --script-path /root/awg/manage_amneziawg.sh \
  --binary-file /tmp/fakebin --no-systemd

# --- сценарий 3: update локальным файлом ---
printf '#!/bin/sh\necho v2\n' > /tmp/fakebin2; chmod +x /tmp/fakebin2
/usr/local/bin/awgram-setup update --binary-file /tmp/fakebin2 --yes --no-systemd
[ "$(sha256sum < /usr/local/bin/awgram)" = "$(sha256sum < /tmp/fakebin2)" ] || fail "update binary content"
[ -f /usr/local/bin/awgram.bak ] || fail "update backup"
[ "$(sha256sum < /usr/local/bin/awgram.bak)" = "$(sha256sum < /tmp/fakebin)" ] || fail "backup is previous binary"

# --- сценарий 4: config флагами ---
/usr/local/bin/awgram-setup config --admins 3 --yes --no-systemd
grep -q '^admin_ids     = \[3\]$' /etc/awgram/config.toml || fail "config admins"
[ -f /etc/awgram/config.toml.bak ] || fail "config backup"
/usr/local/bin/awgram-setup config --token NEWTOKEN --yes --no-systemd
grep -q '^AWGRAM_TOKEN=NEWTOKEN$' /etc/awgram/env || fail "config token"
/usr/local/bin/awgram-setup config --script-path /root/awg/manage_amneziawg.sh --yes --no-systemd
grep -q '^manage_script = "/root/awg/manage_amneziawg.sh"$' /etc/awgram/config.toml || fail "config script"

# --- сценарий 5: status (без сети → latest unknown, не падает) ---
# Вывод сначала захватывается в переменную, а не грепается напрямую из пайпа:
# `grep -q` завершается по первому совпадению и закрывает читающий конец, из-за
# чего awgram-setup может словить SIGPIPE на последующей записи (гонка,
# зависящая от буферизации/планировщика) и pipefail пометит шаг как упавший,
# даже если само совпадение было найдено.
status_out="$(/usr/local/bin/awgram-setup status --no-systemd 2>&1)"
grep -qi 'Installed:' <<<"$status_out" || fail "status output"
grep -qi 'hardened' <<<"$status_out" || fail "status mode"

# --- сценарий 5b: переключение hardened→root мигрирует state.json ---
echo '{"canary":1}' > /var/lib/awgram/state.json
chown awgram:awgram /var/lib/awgram/state.json
bash /repo/install.sh install --yes --mode root --binary-file /tmp/fakebin --no-systemd
[ -f /etc/awgram/state.json ] || fail "state not migrated to /etc/awgram"
grep -q '"canary":1' /etc/awgram/state.json || fail "migrated state content"
[ ! -e /var/lib/awgram/state.json ] || fail "old state file left behind"
grep -q '^state_file = "/etc/awgram/state.json"$' /etc/awgram/config.toml || fail "state_file not switched to root path"
grep -q '^User=' /etc/systemd/system/awgram.service && fail "User= left in unit after switch to root"
[ ! -e /etc/sudoers.d/awgram ] || fail "sudoers left after switch to root"

# --- сценарий 5c: конкурентный запуск блокируется (flock) ---
exec 9>/run/awgram-setup.lock
flock -n 9 || fail "cannot take test lock"
if bash /repo/install.sh config --admins 9 --yes --no-systemd >/dev/null 2>&1; then
  fail "second run must fail while lock is held"
fi
exec 9>&-
grep -q '^admin_ids     = \[3\]$' /etc/awgram/config.toml || fail "config changed despite lock"

# --- сценарий 5d: help упоминает AWGRAM_TOKEN (передача токена без CLI-флага) ---
# захват в переменную, не пайп в grep -q — см. комментарий к сценарию 5 (SIGPIPE)
help_out="$(bash /repo/install.sh help)"
grep -q 'AWGRAM_TOKEN' <<<"$help_out" || fail "help lacks AWGRAM_TOKEN"

# --- сценарий 5e: невалидный токен отклоняется ---
if bash /repo/install.sh config --token 'bad token!' --yes --no-systemd >/dev/null 2>&1; then
  fail "invalid token must be rejected"
fi

# --- сценарий 6: uninstall --purge ---
/usr/local/bin/awgram-setup uninstall --yes --purge --no-systemd
[ ! -e /usr/local/bin/awgram ] || fail "binary not removed"
[ ! -e /etc/awgram ] || fail "cfg dir not purged"
[ ! -e /var/lib/awgram ] || fail "state dir not purged"
[ ! -e /etc/sudoers.d/awgram ] || fail "sudoers not removed"
[ ! -e /etc/systemd/system/awgram.service ] || fail "unit not removed"
[ ! -e /usr/local/bin/awgram-setup ] || fail "setup not removed"
id awgram >/dev/null 2>&1 && fail "user not removed"

# --- сценарий 7: health-check ловит crash-loop сервиса (fake systemctl) ---
# Бот с плохим токеном живёт 1-2с и падает: is-active успевает вернуть active.
# Фейк: active только на первый вызов is-active, затем activating; NRestarts=2.
mkdir -p /tmp/fake-sysd
echo 1 > /tmp/fake-sysd/alive; echo 2 > /tmp/fake-sysd/nrestarts; rm -f /tmp/fake-sysd/n
cat > /usr/local/bin/systemctl <<'FAKE'
#!/bin/bash
d=/tmp/fake-sysd
case "$*" in
  *is-active*)
    n=$(( $(cat "$d/n" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$d/n"
    [ "$n" -le "$(cat "$d/alive")" ] && exit 0 || exit 3 ;;
  *NRestarts*) cat "$d/nrestarts" ;;
esac
exit 0
FAKE
printf '#!/bin/sh\nexit 0\n' > /usr/local/bin/journalctl
chmod +x /usr/local/bin/systemctl /usr/local/bin/journalctl
if bash /repo/install.sh install --yes --lang en --mode root \
     --token TESTTOKEN --admins 1 --binary-file /tmp/fakebin >/dev/null 2>&1; then
  fail "install must fail when service crash-loops"
fi
# здоровый сервис: install проходит
echo 999 > /tmp/fake-sysd/alive; echo 0 > /tmp/fake-sysd/nrestarts; rm -f /tmp/fake-sysd/n
bash /repo/install.sh install --yes --lang en --mode root \
  --token TESTTOKEN --admins 1 --binary-file /tmp/fakebin >/dev/null 2>&1 \
  || fail "install must succeed when service is stable"
rm -f /usr/local/bin/systemctl /usr/local/bin/journalctl
/usr/local/bin/awgram-setup uninstall --yes --purge --no-systemd
[ ! -e /etc/awgram ] || fail "cleanup after scenario 7"

echo "OK: all scenarios passed"
