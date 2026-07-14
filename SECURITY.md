# Политика безопасности / Security Policy

## 🇷🇺 Русский

### Как сообщить об уязвимости

**Не создавайте публичный issue.** Сообщите приватно:

1. GitHub → вкладка **Security** → **Report a vulnerability**
   (private vulnerability reporting), или
2. письмом на **kuraev.e@gmail.com** (тема: `awgram security`).

Отвечаю в течение 7 дней. Исправление — до публичного раскрытия деталей.

### Что считается уязвимостью

awgram управляет VPN-сервером, поэтому особенно критичны:

- обход проверки `admin_ids` (выполнение операций посторонним);
- утечка токена бота, `.conf`-файлов или QR-кодов (в логи, сообщения, файлы);
- инъекция аргументов в вызов `manage_amneziawg.sh`;
- эскалация привилегий в hardened-режиме (sudoers).

### Поддерживаемые версии

Исправления безопасности выходят для последнего релиза (ветка `main`).

---

## 🇬🇧 English

### How to report a vulnerability

**Do not open a public issue.** Report privately:

1. GitHub → **Security** tab → **Report a vulnerability**
   (private vulnerability reporting), or
2. by e-mail to **kuraev.e@gmail.com** (subject: `awgram security`).

I respond within 7 days. The fix ships before details are publicly disclosed.

### What counts as a vulnerability

awgram manages a VPN server, so the following are especially critical:

- bypassing the `admin_ids` check (operations performed by outsiders);
- leakage of the bot token, `.conf` files or QR codes (into logs, messages, files);
- argument injection into `manage_amneziawg.sh` calls;
- privilege escalation in hardened mode (sudoers).

### Supported versions

Security fixes are released for the latest release (the `main` branch).
