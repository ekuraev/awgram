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
      *)             die err_unknown_arg "$1" ;;
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
