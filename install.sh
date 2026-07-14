#!/usr/bin/env bash
# awgram — установщик и менеджер (https://github.com/ekuraev/awgram)
# Установка одной командой:
#   curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh | bash
# После установки доступен как awgram-setup (install|update|config|status|uninstall|help).
set -euo pipefail

SCRIPT_VERSION="1.0.0"
REPO="ekuraev/awgram"
BIN_PATH="/usr/local/bin/awgram"
SETUP_PATH="/usr/local/bin/awgram-setup"
CFG_DIR="/etc/awgram"
CFG_FILE="$CFG_DIR/config.toml"
ENV_FILE="$CFG_DIR/env"
SETUP_CONF="$CFG_DIR/setup.conf"
UNIT_FILE="/etc/systemd/system/awgram.service"
SUDOERS_FILE="/etc/sudoers.d/awgram"
SVC_USER="awgram"

UI_LANG=""; MODE=""; TOKEN=""; ADMINS=""; MANAGE_SCRIPT=""; CLIENTS_DIR=""
PIN_VERSION=""; ASSUME_YES=0; NO_SYSTEMD=0; BINARY_FILE=""; PURGE=0
COMMAND=""; HELP_TOPIC=""; PKG=""; ARCH=""; INSTALLED_VERSION=""; TTY_IN=""

# ---------- i18n ----------
declare -A MSG_RU MSG_EN

MSG_RU[err_not_implemented]="Команда ещё не реализована"
MSG_EN[err_not_implemented]="Command not implemented yet"
MSG_RU[err_unknown_arg]="Неизвестный аргумент: %s (см. help) / Unknown argument: %s (see help)"
MSG_EN[err_unknown_arg]="Неизвестный аргумент: %s (см. help) / Unknown argument: %s (see help)"
MSG_RU[err_bad_lang]="Недопустимое значение --lang: %s (ru|en)"
MSG_EN[err_bad_lang]="Invalid --lang value: %s (ru|en)"
MSG_RU[err_need_root]="Нужны права root: запустите через sudo"
MSG_EN[err_need_root]="Root required: run with sudo"
MSG_RU[err_no_tty]="Нет терминала для вопросов: задайте параметры флагами и добавьте --yes (см. help)"
MSG_EN[err_no_tty]="No terminal for prompts: pass parameters as flags and add --yes (see help)"
MSG_RU[err_os]="Поддерживаются Ubuntu/Debian и RHEL-семейство (AlmaLinux/Rocky/CentOS)"
MSG_EN[err_os]="Supported: Ubuntu/Debian and the RHEL family (AlmaLinux/Rocky/CentOS)"
MSG_RU[err_arch]="Неподдерживаемая архитектура: %s (нужна x86_64 или aarch64)"
MSG_EN[err_arch]="Unsupported architecture: %s (x86_64 or aarch64 required)"
MSG_RU[err_admins]="admin_ids: только цифры через запятую, например 111111111,222222222"
MSG_EN[err_admins]="admin_ids: digits separated by commas, e.g. 111111111,222222222"
MSG_RU[q_deps]="Установить пакеты: %s?"
MSG_EN[q_deps]="Install packages: %s?"
MSG_RU[err_deps]="Без этих пакетов установка невозможна"
MSG_EN[err_deps]="Cannot continue without these packages"
MSG_RU[yn]="[y/N]"
MSG_EN[yn]="[y/N]"
MSG_RU[err_latest]="Не удалось получить последний релиз %s (репо публичный? есть релизы?)"
MSG_EN[err_latest]="Failed to fetch the latest release of %s (is the repo public? any releases?)"
MSG_RU[dl_binary]="Скачиваю %s"
MSG_EN[dl_binary]="Downloading %s"
MSG_RU[err_sha]="Контрольная сумма sha256 не совпала — файл повреждён или подменён"
MSG_EN[err_sha]="sha256 checksum mismatch — the file is corrupted or tampered with"
MSG_RU[err_no_file]="Файл не найден: %s"
MSG_EN[err_no_file]="File not found: %s"
MSG_RU[err_download]="Не удалось скачать %s (релиз существует? ассеты приложены?)"
MSG_EN[err_download]="Failed to download %s (does the release exist? are assets attached?)"
MSG_RU[q_mode]="Режим сервиса: 1) root (проще)  2) hardened (отдельный пользователь + sudoers)"
MSG_EN[q_mode]="Service mode: 1) root (simpler)  2) hardened (dedicated user + sudoers)"
MSG_RU[err_mode]="Недопустимый --mode: %s (root|hardened)"
MSG_EN[err_mode]="Invalid --mode: %s (root|hardened)"
MSG_RU[q_token]="Токен бота от @BotFather (ввод скрыт)"
MSG_EN[q_token]="Bot token from @BotFather (input hidden)"
MSG_RU[err_token]="Токен обязателен (флаг --token или интерактивный ввод)"
MSG_EN[err_token]="Token is required (--token flag or interactive input)"
MSG_RU[q_admins]="Telegram ID администраторов через запятую (узнать: @userinfobot)"
MSG_EN[q_admins]="Comma-separated Telegram admin IDs (get yours: @userinfobot)"
MSG_RU[q_script]="Путь к manage_amneziawg.sh"
MSG_EN[q_script]="Path to manage_amneziawg.sh"
MSG_RU[warn_no_script]="Файл %s не найден — бот не заработает, пока скрипт не появится"
MSG_EN[warn_no_script]="File %s not found — the bot won't work until the script exists"
MSG_RU[q_existing]="awgram уже установлен: 1) обновить  2) перенастроить  3) выйти"
MSG_EN[q_existing]="awgram is already installed: 1) update  2) reconfigure  3) exit"
MSG_RU[svc_ok]="Сервис awgram запущен"
MSG_EN[svc_ok]="awgram service is running"
MSG_RU[svc_failed]="Сервис не запустился — последние строки журнала ниже (частая причина: неверный токен)"
MSG_EN[svc_failed]="Service failed to start — last log lines below (most common cause: invalid token)"
MSG_RU[warn_no_systemd]="systemd недоступен — запуск сервиса пропущен"
MSG_EN[warn_no_systemd]="systemd unavailable — skipping service start"
MSG_RU[warn_self]="Не удалось установить awgram-setup (не критично)"
MSG_EN[warn_self]="Failed to install awgram-setup (not critical)"
MSG_RU[done_install]="Готово! Установлен awgram %s (режим: %s)"
MSG_EN[done_install]="Done! Installed awgram %s (mode: %s)"
MSG_RU[sum_paths]="Конфиг: %s | Токен: %s | Логи: journalctl -u awgram -f | Управление: awgram-setup help"
MSG_EN[sum_paths]="Config: %s | Token: %s | Logs: journalctl -u awgram -f | Manage: awgram-setup help"
MSG_RU[err_sudoers]="Сгенерированный sudoers не прошёл visudo -c — файл не установлен"
MSG_EN[err_sudoers]="Generated sudoers failed visudo -c — file not installed"
MSG_RU[warn_no_cdir]="Каталог %s не существует — ACL не выставлен; после появления каталога: setfacl -R -m u:awgram:rx %s"
MSG_EN[warn_no_cdir]="Directory %s does not exist — ACL not set; once it exists run: setfacl -R -m u:awgram:rx %s"
MSG_RU[warn_acl_failed]="Не удалось выставить ACL на %s (ФС без поддержки ACL?) — выдайте пользователю awgram доступ на чтение вручную: setfacl -R -m u:awgram:rx %s"
MSG_EN[warn_acl_failed]="Failed to set ACL on %s (filesystem without ACL support?) — grant the awgram user read access manually: setfacl -R -m u:awgram:rx %s"
MSG_RU[err_not_installed]="awgram не установлен — сначала выполните install"
MSG_EN[err_not_installed]="awgram is not installed — run install first"
MSG_RU[up_to_date]="Уже последняя версия: %s"
MSG_EN[up_to_date]="Already up to date: %s"
MSG_RU[updated]="Обновлено до %s"
MSG_EN[updated]="Updated to %s"
MSG_RU[rollback]="Откатываюсь на предыдущий бинарник"
MSG_EN[rollback]="Rolling back to the previous binary"
MSG_RU[err_update]="Обновление не удалось — сервис не запустился (выполнен откат)"
MSG_EN[err_update]="Update failed — service did not start (rolled back)"
MSG_RU[cfg_menu]="Что изменить: 1) токен  2) admin_ids  3) путь manage-скрипта  4) показать текущие  5) выход"
MSG_EN[cfg_menu]="What to change: 1) token  2) admin_ids  3) manage-script path  4) show current  5) exit"
MSG_RU[cfg_saved]="Сохранено"
MSG_EN[cfg_saved]="Saved"
MSG_RU[q_restart]="Перезапустить сервис, чтобы применить изменения?"
MSG_EN[q_restart]="Restart the service to apply changes?"
MSG_RU[cfg_current]="Текущие настройки (%s):"
MSG_EN[cfg_current]="Current settings (%s):"
MSG_RU[token_set]="задан"
MSG_EN[token_set]="set"
MSG_RU[token_unset]="не задан"
MSG_EN[token_unset]="not set"
MSG_RU[st_installed]="Установлено: %s | Последний релиз: %s"
MSG_EN[st_installed]="Installed: %s | Latest release: %s"
MSG_RU[st_service]="Сервис: %s | Режим: %s"
MSG_EN[st_service]="Service: %s | Mode: %s"
MSG_RU[st_none]="awgram не установлен"
MSG_EN[st_none]="awgram is not installed"
MSG_RU[q_uninstall]="Удалить awgram (бинарник, сервис, sudoers, пользователь)?"
MSG_EN[q_uninstall]="Remove awgram (binary, service, sudoers, user)?"
MSG_RU[q_purge]="Удалить также конфиг, токен и состояние (%s)?"
MSG_EN[q_purge]="Also remove config, token and state (%s)?"
MSG_RU[uninstalled]="awgram удалён"
MSG_EN[uninstalled]="awgram removed"
MSG_RU[unknown]="неизвестно"
MSG_EN[unknown]="unknown"
MSG_RU[err_bad_path]="Недопустимый путь: %s (символы | и \" не поддерживаются)"
MSG_EN[err_bad_path]="Invalid path: %s (characters | and \" are not supported)"

msg() {
  local key="$1"; shift || true
  local tpl
  if [ "$UI_LANG" = "en" ]; then tpl="${MSG_EN[$key]:-$key}"; else tpl="${MSG_RU[$key]:-$key}"; fi
  # shellcheck disable=SC2059
  printf "$tpl\n" "$@"
}
info() { printf '\033[1;32m==> \033[0m' >&2; msg "$@" >&2; }
warn() { printf '\033[1;33m !  \033[0m' >&2; msg "$@" >&2; }
die()  { printf '\033[1;31mERR \033[0m' >&2; msg "$@" >&2; exit 1; }

# ---------- утилиты окружения ----------
ensure_root() { [ "$(id -u)" = "0" ] || die err_need_root; }

init_tty() {
  if [ -t 0 ]; then TTY_IN="/dev/stdin"
  elif [ -r /dev/tty ] && [ -w /dev/tty ]; then TTY_IN="/dev/tty"
  else TTY_IN=""
  fi
}

choose_language() {
  [ -n "$UI_LANG" ] && return 0
  if [ -f "$SETUP_CONF" ]; then
    UI_LANG="$(sed -n 's/^LANG=//p' "$SETUP_CONF" | head -1)"
    [ -n "$UI_LANG" ] && return 0
  fi
  if [ "$ASSUME_YES" = 1 ] || [ -z "$TTY_IN" ]; then UI_LANG="en"; return 0; fi
  printf '1) Русский  2) English\nЯзык / Language [1/2]: ' >&2
  local a=""; IFS= read -r a <"$TTY_IN" || true
  case "$a" in 2*|[eE]*) UI_LANG="en" ;; *) UI_LANG="ru" ;; esac
}

detect_os() {
  [ -r /etc/os-release ] || die err_os
  # shellcheck disable=SC1091
  . /etc/os-release
  case " ${ID:-} ${ID_LIKE:-} " in
    *" debian "*|*" ubuntu "*) PKG="apt" ;;
    *" rhel "*|*" fedora "*|*" centos "*)
      if command -v dnf >/dev/null 2>&1; then PKG="dnf"; else PKG="yum"; fi ;;
    *) die err_os ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64)  ARCH="amd64" ;;
    aarch64) ARCH="arm64" ;;
    *) die err_arch "$(uname -m)" ;;
  esac
}

is_systemd() { [ "$NO_SYSTEMD" != 1 ] && command -v systemctl >/dev/null 2>&1; }

ask() { # $1=msg-ключ, $2=default; stdout=ответ
  local key="$1" def="${2:-}" ans=""
  if [ "$ASSUME_YES" = 1 ] || [ -z "$TTY_IN" ]; then
    [ -n "$def" ] && { printf '%s\n' "$def"; return 0; }
    die err_no_tty
  fi
  msg "$key" >&2
  if [ -n "$def" ]; then printf '  [%s]: ' "$def" >&2; else printf '  : ' >&2; fi
  IFS= read -r ans <"$TTY_IN" || true
  printf '%s\n' "${ans:-$def}"
}

ask_secret() { # $1=msg-ключ; stdout=ответ (ввод скрыт)
  local key="$1" ans=""
  if [ "$ASSUME_YES" = 1 ] || [ -z "$TTY_IN" ]; then die err_no_tty; fi
  msg "$key" >&2; printf '  : ' >&2
  IFS= read -rs ans <"$TTY_IN" || true
  printf '\n' >&2
  printf '%s\n' "$ans"
}

confirm() { # 0=да; --yes → всегда да
  [ "$ASSUME_YES" = 1 ] && return 0
  [ -z "$TTY_IN" ] && return 1
  msg "$@" >&2; printf '  %s ' "$(msg yn)" >&2
  local a=""; IFS= read -r a <"$TTY_IN" || true
  case "$a" in [yYдД]*) return 0 ;; *) return 1 ;; esac
}

validate_admins() { [[ "$ADMINS" =~ ^[0-9]+(,[0-9]+)*$ ]]; }

validate_path() { # $1=путь; 0 если безопасен для set_toml/конфига
  case "$1" in *'|'*|*'"'*) return 1 ;; *) return 0 ;; esac
}

load_setup_conf() {
  [ -f "$SETUP_CONF" ] || return 0
  local v
  v="$(sed -n 's/^LANG=//p' "$SETUP_CONF" | head -1)";           [ -n "$UI_LANG" ] || UI_LANG="$v"
  v="$(sed -n 's/^MODE=//p' "$SETUP_CONF" | head -1)";           [ -n "$MODE" ] || MODE="$v"
  v="$(sed -n 's/^VERSION=//p' "$SETUP_CONF" | head -1)";        INSTALLED_VERSION="$v"
  v="$(sed -n 's/^MANAGE_SCRIPT=//p' "$SETUP_CONF" | head -1)";  [ -n "$MANAGE_SCRIPT" ] || MANAGE_SCRIPT="$v"
  v="$(sed -n 's/^CLIENTS_DIR=//p' "$SETUP_CONF" | head -1)";    [ -n "$CLIENTS_DIR" ] || CLIENTS_DIR="$v"
}

save_setup_conf() {
  mkdir -p "$CFG_DIR"
  cat > "$SETUP_CONF" <<EOF
LANG=$UI_LANG
MODE=$MODE
VERSION=$INSTALLED_VERSION
MANAGE_SCRIPT=$MANAGE_SCRIPT
CLIENTS_DIR=$CLIENTS_DIR
EOF
}

ensure_deps() {
  local pkgs=()
  command -v curl >/dev/null 2>&1 || pkgs+=(curl ca-certificates)
  if [ "$MODE" = "hardened" ]; then
    command -v visudo  >/dev/null 2>&1 || pkgs+=(sudo)
    command -v setfacl >/dev/null 2>&1 || pkgs+=(acl)
  fi
  [ "${#pkgs[@]}" -eq 0 ] && return 0
  confirm q_deps "${pkgs[*]}" || die err_deps
  case "$PKG" in
    apt) DEBIAN_FRONTEND=noninteractive apt-get update -qq >&2
         DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${pkgs[@]}" >&2 ;;
    dnf) dnf install -y -q "${pkgs[@]}" >&2 ;;
    yum) yum install -y -q "${pkgs[@]}" >&2 ;;
  esac
}

# ---------- релизы ----------
fetch_latest_tag() {
  local tag
  tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)" || true
  [ -n "$tag" ] || die err_latest "$REPO"
  printf '%s\n' "$tag"
}

fetch_binary() { # $1=tag; stdout=путь staged-файла
  local tag="$1" tmpd url
  tmpd="$(mktemp -d)"
  if [ -n "$BINARY_FILE" ]; then
    [ -f "$BINARY_FILE" ] || die err_no_file "$BINARY_FILE"
    cp "$BINARY_FILE" "$tmpd/awgram-linux-$ARCH"
  else
    url="https://github.com/$REPO/releases/download/$tag/awgram-linux-$ARCH"
    info dl_binary "$url"
    curl -fSL --progress-bar -o "$tmpd/awgram-linux-$ARCH" "$url" >&2 || die err_download "$url"
    curl -fsSL -o "$tmpd/awgram-linux-$ARCH.sha256" "$url.sha256" || die err_download "$url.sha256"
    (cd "$tmpd" && sha256sum -c "awgram-linux-$ARCH.sha256" >/dev/null 2>&1) || die err_sha
  fi
  printf '%s\n' "$tmpd/awgram-linux-$ARCH"
}

install_binary() { # $1=staged
  [ -f "$BIN_PATH" ] && cp -f "$BIN_PATH" "$BIN_PATH.bak"
  install -m 755 "$1" "$BIN_PATH.new"
  mv -f "$BIN_PATH.new" "$BIN_PATH"
}

# ---------- конфигурация ----------
write_env_token() {
  mkdir -p "$CFG_DIR"
  ( umask 077; printf 'AWGRAM_TOKEN=%s\n' "$TOKEN" > "$ENV_FILE" )
  chmod 600 "$ENV_FILE"
}

write_config() {
  mkdir -p "$CFG_DIR"
  local sudo_prefix=""
  [ "$MODE" = "hardened" ] && sudo_prefix="sudo"
  cat > "$CFG_FILE" <<EOF
# Сгенерировано awgram-setup / Generated by awgram-setup
bot_token     = ""                              # токен в $ENV_FILE (AWGRAM_TOKEN) / token lives in $ENV_FILE
admin_ids     = [${ADMINS//,/, }]
manage_script = "$MANAGE_SCRIPT"
clients_dir   = "$CLIENTS_DIR"
sudo_prefix   = "$sudo_prefix"
op_timeout_secs = 60
state_file = "$CFG_DIR/state.json"
EOF
  chmod 640 "$CFG_FILE"
}

install_unit() {
  local user_line=""
  [ "$MODE" = "hardened" ] && user_line="User=$SVC_USER"
  cat > "$UNIT_FILE" <<EOF
[Unit]
Description=awgram — Telegram bot for AmneziaWG
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
${user_line}
ExecStart=$BIN_PATH
Environment=AWGRAM_CONFIG=$CFG_FILE
EnvironmentFile=-$ENV_FILE
Restart=on-failure
RestartSec=5
NoNewPrivileges=false

[Install]
WantedBy=multi-user.target
EOF
}

wait_active() {
  local i
  for i in 1 2 3 4 5; do
    sleep 1
    systemctl is-active --quiet awgram && return 0
  done
  return 1
}

start_service() {
  if ! is_systemd; then warn warn_no_systemd; return 0; fi
  systemctl daemon-reload
  systemctl enable --now awgram >/dev/null 2>&1 || true
  if wait_active; then info svc_ok; return 0; fi
  warn svc_failed
  journalctl -u awgram -n 20 --no-pager >&2 || true
  return 1
}

self_install() {
  local src="${BASH_SOURCE[0]:-}"
  if [ -n "$src" ] && [ -f "$src" ]; then
    [ "$src" -ef "$SETUP_PATH" ] 2>/dev/null || install -m 755 "$src" "$SETUP_PATH"
  else
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/install.sh" -o "$SETUP_PATH.new" 2>/dev/null \
      && install -m 755 "$SETUP_PATH.new" "$SETUP_PATH" && rm -f "$SETUP_PATH.new" \
      || warn warn_self
  fi
}

summary() {
  info done_install "$INSTALLED_VERSION" "$MODE"
  info sum_paths "$CFG_FILE" "$ENV_FILE"
}

cmd_install() {
  ensure_root; init_tty; choose_language; detect_os; detect_arch
  # повторная установка
  if [ -f "$SETUP_CONF" ] && [ -x "$BIN_PATH" ]; then
    if [ "$ASSUME_YES" != 1 ] && [ -n "$TTY_IN" ]; then
      msg q_existing >&2; printf '  [1/2/3]: ' >&2
      local a=""; IFS= read -r a <"$TTY_IN" || true
      case "$a" in
        1) cmd_update; return 0 ;;
        2) load_setup_conf ;;
        *) return 0 ;;
      esac
    else
      load_setup_conf
    fi
  fi
  # параметры
  if [ -z "$MODE" ]; then
    local m; m="$(ask q_mode "1")"
    case "$m" in 2*|h*) MODE="hardened" ;; *) MODE="root" ;; esac
  fi
  case "$MODE" in root|hardened) ;; *) die err_mode "$MODE" ;; esac
  ensure_deps
  [ -n "$TOKEN" ] || TOKEN="$(ask_secret q_token)"
  [ -n "$TOKEN" ] || die err_token
  [ -n "$ADMINS" ] || ADMINS="$(ask q_admins "")"
  validate_admins || die err_admins
  [ -n "$MANAGE_SCRIPT" ] || MANAGE_SCRIPT="$(ask q_script "/root/awg/manage_amneziawg.sh")"
  validate_path "$MANAGE_SCRIPT" || die err_bad_path "$MANAGE_SCRIPT"
  [ -f "$MANAGE_SCRIPT" ] || warn warn_no_script "$MANAGE_SCRIPT"
  [ -n "$CLIENTS_DIR" ] || CLIENTS_DIR="$(dirname "$MANAGE_SCRIPT")"
  validate_path "$CLIENTS_DIR" || die err_bad_path "$CLIENTS_DIR"
  # бинарник
  local tag staged
  if [ -n "$PIN_VERSION" ]; then tag="$PIN_VERSION"
  elif [ -n "$BINARY_FILE" ]; then tag="local"
  else tag="$(fetch_latest_tag)"; fi
  staged="$(fetch_binary "$tag")"
  install_binary "$staged"
  # конфигурация и запуск
  write_config
  write_env_token
  [ "$MODE" = "hardened" ] && setup_hardened
  install_unit
  INSTALLED_VERSION="$tag"
  save_setup_conf
  self_install
  start_service || { msg svc_failed >&2; exit 1; }
  summary
}

# ---------- help ----------
help_ru() {
  cat <<'EOF'
awgram-setup — установка и управление awgram (Telegram-бот для AmneziaWG)

Использование:
  install.sh | awgram-setup [КОМАНДА] [ФЛАГИ]

Команды:
  install     установить бота (по умолчанию; интерактивно или флагами)
  update      обновить бинарник до последнего релиза (и сам awgram-setup)
  config      изменить параметры: токен, admin_ids, путь к manage-скрипту
  status      версия, состояние сервиса, режим, пути
  uninstall   удалить бота (конфиг — с подтверждением или --purge)
  help [cmd]  эта справка или справка по команде

Флаги (install; для config действуют --token/--admins/--script-path):
  --lang ru|en          язык интерфейса (сохраняется)
  --mode root|hardened  режим сервиса: от root или отдельный пользователь+sudoers
  --token TOKEN         токен бота от @BotFather (пишется в /etc/awgram/env)
  --admins 1,2,3        Telegram ID администраторов через запятую
  --script-path PATH    путь к manage_amneziawg.sh (по умолчанию /root/awg/manage_amneziawg.sh)
  --clients-dir PATH    каталог client-конфигов (по умолчанию каталог manage-скрипта)
  --version vX.Y.Z      установить конкретный релиз вместо последнего
  --yes | -y            без вопросов (для автоматизации; недостающий параметр — ошибка)
  --purge               (uninstall) удалить также конфиг и состояние

Примеры:
  curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh | bash
  curl -fsSL ... | bash -s -- install --lang ru --mode hardened --token 'X' --admins 1 --yes
  awgram-setup config --admins 1,2
EOF
}
help_en() {
  cat <<'EOF'
awgram-setup — install and manage awgram (Telegram bot for AmneziaWG)

Usage:
  install.sh | awgram-setup [COMMAND] [FLAGS]

Commands:
  install     install the bot (default; interactive or via flags)
  update      update the binary to the latest release (and awgram-setup itself)
  config      change settings: token, admin_ids, manage-script path
  status      version, service state, mode, paths
  uninstall   remove the bot (config removed only with confirmation or --purge)
  help [cmd]  this help or per-command help

Flags (install; config accepts --token/--admins/--script-path):
  --lang ru|en          interface language (persisted)
  --mode root|hardened  service mode: run as root or dedicated user + sudoers
  --token TOKEN         bot token from @BotFather (written to /etc/awgram/env)
  --admins 1,2,3        comma-separated Telegram admin IDs
  --script-path PATH    path to manage_amneziawg.sh (default /root/awg/manage_amneziawg.sh)
  --clients-dir PATH    client-config dir (default: the manage-script directory)
  --version vX.Y.Z      install a specific release instead of the latest
  --yes | -y            no questions (for automation; a missing parameter is an error)
  --purge               (uninstall) also remove config and state

Examples:
  curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh | bash
  curl -fsSL ... | bash -s -- install --lang en --mode hardened --token 'X' --admins 1 --yes
  awgram-setup config --admins 1,2
EOF
}
cmd_help() {
  # без выбранного языка печатаем обе версии
  case "$UI_LANG" in
    ru) help_ru ;;
    en) help_en ;;
    *)  help_ru; echo; help_en ;;
  esac
}

# ---------- hardened mode setup ----------
setup_hardened() {
  if ! id -u "$SVC_USER" >/dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SVC_USER" 2>/dev/null \
      || useradd --system --no-create-home --shell /sbin/nologin "$SVC_USER"
  fi
  local tmp; tmp="$(mktemp)"
  printf '%s ALL=(root) NOPASSWD: %s\n' "$SVC_USER" "$MANAGE_SCRIPT" > "$tmp"
  chmod 440 "$tmp"
  visudo -c -f "$tmp" >/dev/null 2>&1 || { rm -f "$tmp"; die err_sudoers; }
  mv -f "$tmp" "$SUDOERS_FILE"
  if [ -d "$CLIENTS_DIR" ]; then
    { setfacl -R -m "u:$SVC_USER:rx" "$CLIENTS_DIR" \
        && setfacl -R -d -m "u:$SVC_USER:rx" "$CLIENTS_DIR"; } 2>/dev/null \
      || warn warn_acl_failed "$CLIENTS_DIR" "$CLIENTS_DIR"
  else
    warn warn_no_cdir "$CLIENTS_DIR" "$CLIENTS_DIR"
  fi
}

# ---------- заглушки (заменяются задачами 6-8) ----------
cmd_update() {
  ensure_root; init_tty; load_setup_conf; choose_language; detect_os; detect_arch
  [ -x "$BIN_PATH" ] || die err_not_installed
  local tag staged
  if [ -n "$PIN_VERSION" ]; then tag="$PIN_VERSION"
  elif [ -n "$BINARY_FILE" ]; then tag="local"
  else tag="$(fetch_latest_tag)"; fi
  if [ "$tag" = "$INSTALLED_VERSION" ] && [ -z "$BINARY_FILE" ] && [ -z "$PIN_VERSION" ]; then
    info up_to_date "$tag"; return 0
  fi
  staged="$(fetch_binary "$tag")"
  install_binary "$staged"
  if is_systemd; then
    systemctl restart awgram 2>/dev/null || true
    if ! wait_active; then
      warn svc_failed
      journalctl -u awgram -n 20 --no-pager >&2 || true
      if [ -f "$BIN_PATH.bak" ]; then
        warn rollback
        mv -f "$BIN_PATH.bak" "$BIN_PATH"
        systemctl restart awgram 2>/dev/null || true
      fi
      die err_update
    fi
  fi
  INSTALLED_VERSION="$tag"
  save_setup_conf
  info updated "$tag"
  # самообновление awgram-setup (не критично при отказе)
  curl -fsSL "https://raw.githubusercontent.com/$REPO/main/install.sh" -o "$SETUP_PATH.new" 2>/dev/null \
    && install -m 755 "$SETUP_PATH.new" "$SETUP_PATH" && rm -f "$SETUP_PATH.new" \
    || rm -f "$SETUP_PATH.new" 2>/dev/null || true
}
set_toml() { # $1=ключ, $2=готовое toml-значение (без экранирования | в значении)
  cp -f "$CFG_FILE" "$CFG_FILE.bak"
  sed -i "s|^\($1[[:space:]]*=\).*|\1 $2|" "$CFG_FILE"
}

show_current() {
  msg cfg_current "$CFG_FILE" >&2
  grep -E '^(admin_ids|manage_script|clients_dir|sudo_prefix)' "$CFG_FILE" >&2 || true
  if [ -s "$ENV_FILE" ]; then printf 'token: %s\n' "$(msg token_set)" >&2
  else printf 'token: %s\n' "$(msg token_unset)" >&2; fi
}

maybe_restart() {
  is_systemd || return 0
  confirm q_restart || return 0
  systemctl restart awgram 2>/dev/null || true
  wait_active && info svc_ok || { warn svc_failed; journalctl -u awgram -n 20 --no-pager >&2 || true; }
}

cmd_config() {
  ensure_root; init_tty; load_setup_conf; choose_language
  [ -f "$CFG_FILE" ] || die err_not_installed
  local changed=0
  if [ -n "$TOKEN" ]; then write_env_token; changed=1; fi
  if [ -n "$ADMINS" ]; then
    validate_admins || die err_admins
    set_toml admin_ids "[${ADMINS//,/, }]"; changed=1
  fi
  if [ -n "$MANAGE_SCRIPT" ]; then
    validate_path "$MANAGE_SCRIPT" || die err_bad_path "$MANAGE_SCRIPT"
    [ -f "$MANAGE_SCRIPT" ] || warn warn_no_script "$MANAGE_SCRIPT"
    set_toml manage_script "\"$MANAGE_SCRIPT\""
    save_setup_conf; changed=1
  fi
  if [ "$changed" = 0 ]; then
    [ -n "$TTY_IN" ] || die err_no_tty
    while true; do
      local c; c="$(ask cfg_menu "5")"
      case "$c" in
        1) TOKEN="$(ask_secret q_token)"; [ -n "$TOKEN" ] && { write_env_token; changed=1; info cfg_saved; } ;;
        2) ADMINS="$(ask q_admins "")"; validate_admins || { warn err_admins; continue; }
           set_toml admin_ids "[${ADMINS//,/, }]"; changed=1; info cfg_saved ;;
        3) MANAGE_SCRIPT="$(ask q_script "")"; [ -n "$MANAGE_SCRIPT" ] || continue
           validate_path "$MANAGE_SCRIPT" || { warn err_bad_path "$MANAGE_SCRIPT"; continue; }
           [ -f "$MANAGE_SCRIPT" ] || warn warn_no_script "$MANAGE_SCRIPT"
           set_toml manage_script "\"$MANAGE_SCRIPT\""; save_setup_conf; changed=1; info cfg_saved ;;
        4) show_current ;;
        *) break ;;
      esac
    done
  else
    info cfg_saved
  fi
  [ "$changed" = 1 ] && maybe_restart
  return 0
}

cmd_status() {
  init_tty; load_setup_conf; choose_language
  if [ ! -x "$BIN_PATH" ]; then msg st_none >&2; return 0; fi
  local latest svc
  latest="$(fetch_latest_tag 2>/dev/null)" || latest=""
  [ -n "$latest" ] || latest="$(msg unknown)"
  if is_systemd; then svc="$(systemctl is-active awgram 2>/dev/null || true)"; else svc="$(msg unknown)"; fi
  msg st_installed "${INSTALLED_VERSION:-$(msg unknown)}" "$latest" >&2
  msg st_service "${svc:-$(msg unknown)}" "${MODE:-$(msg unknown)}" >&2
  [ -r "$CFG_FILE" ] && show_current || true
}

cmd_uninstall() {
  ensure_root; init_tty; load_setup_conf; choose_language
  confirm q_uninstall || return 0
  if is_systemd; then
    systemctl disable --now awgram >/dev/null 2>&1 || true
  fi
  rm -f "$UNIT_FILE" "$SUDOERS_FILE" "$BIN_PATH" "$BIN_PATH.bak"
  is_systemd && systemctl daemon-reload || true
  id -u "$SVC_USER" >/dev/null 2>&1 && userdel "$SVC_USER" 2>/dev/null || true
  if [ "$PURGE" = 1 ]; then
    rm -rf "$CFG_DIR"
  elif [ "$ASSUME_YES" != 1 ] && confirm q_purge "$CFG_DIR"; then
    rm -rf "$CFG_DIR"
  fi
  rm -f "$SETUP_PATH"
  info uninstalled
}

# ---------- парсинг аргументов и диспетчер ----------
main() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --lang)        UI_LANG="${2:?--lang}"; shift 2
                     case "$UI_LANG" in ru|en) ;; *) die err_bad_lang "$UI_LANG" ;; esac ;;
      --mode)        MODE="${2:?--mode}"; shift 2 ;;
      --token)       TOKEN="${2:?--token}"; shift 2 ;;
      --admins)      ADMINS="${2:?--admins}"; shift 2 ;;
      --script-path) MANAGE_SCRIPT="${2:?--script-path}"; shift 2 ;;
      --clients-dir) CLIENTS_DIR="${2:?--clients-dir}"; shift 2 ;;
      --version)     PIN_VERSION="${2:?--version}"; shift 2 ;;
      --repo)        REPO="${2:?--repo}"; shift 2 ;;
      --binary-file) BINARY_FILE="${2:?--binary-file}"; shift 2 ;;
      --yes|-y)      ASSUME_YES=1; shift ;;
      --no-systemd)  NO_SYSTEMD=1; shift ;;
      --purge)       PURGE=1; shift ;;
      -h|--help)     COMMAND="help"; shift ;;
      install|update|config|status|uninstall) COMMAND="$1"; shift ;;
      help)          COMMAND="help"; shift; HELP_TOPIC="${1:-}"; [ $# -gt 0 ] && shift ;;
      *)             die err_unknown_arg "$1" "$1" ;;
    esac
  done
  : "${COMMAND:=install}"
  case "$COMMAND" in
    install)   cmd_install ;;
    update)    cmd_update ;;
    config)    cmd_config ;;
    status)    cmd_status ;;
    uninstall) cmd_uninstall ;;
    help)      cmd_help ;;
  esac
}

main "$@"
