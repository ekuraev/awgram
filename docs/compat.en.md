# AmneziaWG installer compatibility

[Русская версия](compat.md)

awgram is a layer on top of `manage_amneziawg.sh` from
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
and depends directly on its `--json` interface. This page lists exactly
what is used and how each installer release affected (or did not affect)
the bot.

## In short

| | Version |
|---|---|
| Supported (`--json` contract verified) | [v5.31.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.31.0) |
| Minimum | [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0) |

v5.20.x and older are not supported: the bot relies on the extended
`--json` interface for management commands introduced in v5.21.0.

Subcommands used, all with `--json`: `add`, `remove`, `list`, `stats`,
`regen`, `modify`, `backup`, `restore`, `check`, `restart`, `repair-module`.

## Optional `add --allowed-ips`

The flag from [Issue #253](https://github.com/bivlked/amneziawg-installer/issues/253)
sets per-client routes by the same call that creates the client. The bot
detects the flag from the script's own help (`--help`, which changes
nothing) rather than from a version number: the release carrying the flag
is not known in advance, and on a half-updated server the number and
reality disagree.

- Flag present — the client is created with the right routes right away;
  there is never an intermediate "client exists, routes missing" state.
- Flag absent — the old `add` + `modify` path runs before files are
  delivered; the minimum version stays the same.
- Bulk creation has no fallback: `modify` would have to run once per
  client, so without the flag the routes step is hidden there.

## Installer release history

None of the v5.21.1–v5.31.0 releases broke the JSON contract. All new
messages go to stderr; the `--json` envelopes on stdout are unchanged.

| Version | What changed | Effect on the bot |
|---|---|---|
| v5.21.1, v5.21.2 | Validation bugfixes | None |
| v5.22.0 | `regen`/`check` warn about `awgsetup_cfg.init` drift | None (stderr) |
| v5.23.0 | Installer only: kernel module on older kernels | None |
| v5.24.0 | Additive `module.version` field in `check --json` | None |
| v5.25.0 | New warnings only | None (stderr) |
| v5.26.0 | Cascade routing script, diagnostic report | None |
| v5.27.0 | Installer only: package-removal consent | None |
| v5.27.1 | `modify`/`regen` normalize `AllowedIPs`/`DNS` lists to the canonical "a, b, c" form; `regen` no longer collapses them | None: the `modify` reply still echoes `value` as sent, and the bot already sends lists in canonical form |
| v5.28.0, v5.29.0 | Installer only (boot-critical package protection, key masking in the report), release signing, docs | None: `manage_amneziawg.sh` changed nothing but its version number |
| v5.30.0 | Every interface call is bounded by a timeout; an unread state is no longer reported as measured | On a failed read `list --json` leaves clients at `no_data` — the bot marks them yellow; `check --json` may return an empty `interface.addresses` — the VPN subnet preset is hidden and bulk creation reports capacity as unavailable |
| v5.31.0 | Full tunnel is decided by route coverage; the default "Amnezia" mode gets `::/0`; `modify` warns about a full tunnel without `::/0` | None: the "all traffic" preset sends `0.0.0.0/0, ::/0`, so the warning never fires |

## How a new installer release is verified

When the installer ships a new release, the `manage_amneziawg.sh` diff is
reviewed: `--json` envelopes of the subcommands in use, exit codes, new
fields and warnings. If the contract is intact, only this table and the
README badge are updated; if it changed, the fix lands in the CHANGELOG as
compatibility with that specific version.
