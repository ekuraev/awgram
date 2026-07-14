# Как внести вклад / Contributing

## 🇷🇺 Русский

### Сборка и проверка

Нужен стабильный Rust (edition 2021):

```bash
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

CI гоняет ровно эти четыре команды — перед PR убедитесь, что все проходят.
Статический бинарник для VPS: `./scripts/build-musl.sh` (нужен Docker).

### Стиль коммитов

[Conventional Commits](https://www.conventionalcommits.org/ru/):
`feat(bot): …`, `fix(vpn): …`, `docs(readme): …`. Область — модуль
(`bot`, `vpn`, `model`, `i18n`, `config`, …). Язык описания — русский или английский.

### Pull Request

- Ветка от `main`, один PR — одно логическое изменение.
- Новая логика — с тестами (`#[cfg(test)]` рядом с кодом, интеграционные — в `tests/`).
- Не включайте в диффы токены, `.conf`-файлы и QR-коды.

### Сообщения об ошибках

Используйте шаблоны issues — там перечислено, какие данные нужны
(версия, ОС, способ установки AmneziaWG, шаги, логи).

---

## 🇬🇧 English

### Building and checks

You need stable Rust (2021 edition):

```bash
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

CI runs exactly these four commands — make sure they all pass before opening
a PR. Static VPS binary: `./scripts/build-musl.sh` (requires Docker).

### Commit style

[Conventional Commits](https://www.conventionalcommits.org/):
`feat(bot): …`, `fix(vpn): …`, `docs(readme): …`. The scope is a module name
(`bot`, `vpn`, `model`, `i18n`, `config`, …). Commit descriptions may be in
Russian or English.

### Pull Request

- Branch off `main`; one PR — one logical change.
- New logic comes with tests (`#[cfg(test)]` next to the code, integration
  tests in `tests/`).
- Never include bot tokens, client `.conf` files or QR codes in diffs.

### Bug reports

Use the issue templates — they list the data needed
(version, OS, AmneziaWG install method, steps, logs).
