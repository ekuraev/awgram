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

# ---------- заглушки (заменяются задачами 4-8) ----------
cmd_install()   { die err_not_implemented; }
cmd_update()    { die err_not_implemented; }
cmd_config()    { die err_not_implemented; }
cmd_status()    { die err_not_implemented; }
cmd_uninstall() { die err_not_implemented; }

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
