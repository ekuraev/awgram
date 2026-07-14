# Политика безопасности / Security Policy

🇷🇺 Русский · [🇬🇧 English](#english)

## Как сообщить об уязвимости

**Не создавайте публичный issue.** Сообщите приватно:

1. GitHub → вкладка **Security** → **Report a vulnerability**
   (private vulnerability reporting), или
2. письмом на **kuraev.e@gmail.com** (тема: `awgram security`).

Отвечаю в течение 7 дней. Исправление — до публичного раскрытия деталей.

## Что считается уязвимостью

awgram управляет VPN-сервером, поэтому особенно критичны:

- обход проверки `admin_ids` (выполнение операций посторонним);
- утечка токена бота, `.conf`-файлов или QR-кодов (в логи, сообщения, файлы);
- инъекция аргументов в вызов `manage_amneziawg.sh`;
- эскалация привилегий в hardened-режиме (sudoers).

## Поддерживаемые версии

Исправления безопасности выходят для последнего релиза (ветка `main`).

---

## English

**Do not open a public issue.** Report privately via GitHub → **Security** →
**Report a vulnerability**, or e-mail **kuraev.e@gmail.com** (subject: `awgram security`).
Expect a response within 7 days; fixes ship before details are disclosed.

In scope: `admin_ids` auth bypass, leakage of the bot token / client `.conf` /
QR codes, argument injection into `manage_amneziawg.sh` calls, privilege
escalation in hardened mode (sudoers). Security fixes target the latest
release (`main` branch).
