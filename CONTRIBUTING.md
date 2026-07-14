# Как внести вклад / Contributing

🇷🇺 Русский · [🇬🇧 English](#english)

## Сборка и проверка

Нужен стабильный Rust (edition 2021):

```bash
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

CI гоняет ровно эти четыре команды — перед PR убедитесь, что все проходят.
Статический бинарник для VPS: `./scripts/build-musl.sh` (нужен Docker).

## Стиль коммитов

[Conventional Commits](https://www.conventionalcommits.org/ru/):
`feat(bot): …`, `fix(vpn): …`, `docs(readme): …`. Область — модуль
(`bot`, `vpn`, `model`, `i18n`, `config`, …). Язык описания — русский или английский.

## Pull Request

- Ветка от `main`, один PR — одно логическое изменение.
- Новая логика — с тестами (`#[cfg(test)]` рядом с кодом, интеграционные — в `tests/`).
- Не включайте в диффы токены, `.conf`-файлы и QR-коды.

## Сообщения об ошибках

Используйте шаблоны issues — там перечислено, какие данные нужны
(версия, ОС, способ установки AmneziaWG, шаги, логи).

---

## English

Stable Rust (2021 edition) required. Before opening a PR make sure all four
CI commands pass:

```bash
cargo build && cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Static VPS binary: `./scripts/build-musl.sh` (requires Docker).

Commits follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat(bot): …`, `fix(vpn): …`); scope = module name. Russian or English is fine.

One PR — one logical change, branched off `main`. New logic needs tests.
Never include bot tokens, client `.conf` files or QR codes in diffs or issues.
