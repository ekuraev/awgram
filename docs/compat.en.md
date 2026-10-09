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
| Supported (`--json` contract verified) | [v5.37.1](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.37.1) |
| Minimum | [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0) |

v5.20.x and older are not supported: the bot relies on the extended
`--json` interface for management commands introduced in v5.21.0.

Subcommands used, all with `--json`: `add`, `remove`, `list`, `stats`,
`regen`, `modify`, `backup`, `restore`, `check`, `restart`, `repair-module`.

## Optional `add --allowed-ips`

The flag from [Issue #253](https://github.com/bivlked/amneziawg-installer/issues/253)
(shipped in installer v5.32.0) sets per-client routes by the same call that creates the client. The bot
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

Releases v5.21.1–v5.37.0 did not break the JSON contract: all new messages
went to stderr and the `--json` envelopes on stdout were unchanged. v5.37.1
is the first where a field of an existing envelope changed its type (`null`
instead of `false` in `repair-module --json`), see its row below.

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
| v5.32.0 | `add --allowed-ips=LIST` — per-client routes at creation ([Issue #253](https://github.com/bivlked/amneziawg-installer/issues/253)), the `created` entry of `add --json` gains an additive `allowed_ips` field; `list --json` reports `expires_at` and `expires_at_error` in every record; an expiry marker with a leading zero no longer deletes the client; protocol generation marker `AWG_PROTOCOL` in `awgsetup_cfg.init`, `restore` warns when the generation differs | The flag the bot has detected via `--help` since v0.10.0 is now in a release: the client is created with its routes in one call, with no intermediate `modify`. On an IPv6-enabled server the script appends an IPv6 part to an IPv4-only list (the VPN subnet, or `::/0` for a full tunnel) — unlike the old `modify`, which wrote the list as is; the applied value comes back in `allowed_ips`. `expires_at`/`expires_at_error` are the source of expiry for the list and the card (an unreadable marker shows as ⚠️), on an installer without the fields the bot still reads `expiry/<name>`; `allowed_ips` from the `add` reply is shown on the "Done" screen; the `restore` warning goes to stderr |
| v5.33.0 | Installer and `I1` generator only: `--protocol` flag, `I1` shaped as a DNS reply | None |
| v5.34.0, v5.34.1 | Full tunnel is the default mode; `I1`–`I5` are checked when profiles are issued; `modify` rebuilds `.png`/`.vpnuri` and returns additive `qr`/`vpnuri` | None: `add`/`regen` refusals come as `status:"error"`, the QR and link are sent only if the file exists |
| v5.35.0 | `add`/`regen`/`modify` refuse when `HeaderProtectionKey` disagrees with the `AWG_PROTOCOL` marker; keys are masked in `check`/`show` output and traces | A `modify` refusal comes as an `ok:false` error envelope; before awgram v0.12.1 the bot took it for success — fixed |
| v5.36.0–v5.36.2 | Additive `protocol`/`protocol_error` in `check --json`; secrets hidden in the service status; since v5.36.2 a list-based full tunnel gets `2000::/3` plus a "sink" address instead of `::/0` (the Windows client reaches the LAN again) | The "all traffic" preset (`0.0.0.0/0, ::/0`) is left alone. Since awgram v0.12.2 the "exclude from VPN" mode sends `2000::/3` instead of `::/0` on v5.36.2+ — the installer adds the sink address itself; the routes screen recognises these lists and the mode 2 list (as "exclude all local") |
| v5.37.0 | Default client DNS (`CLIENT_DNS`) in the install config; an empty `--expires=` is an error; `add`/`restore` handle expiry markers more strictly | None: the bot passes `--expires` only with a non-empty validated value, refusals come as `status:"error"`/`rolled_back` |
| v5.37.1 | `repair-module --json` goes through the module helper (`amneziawg-ensure-module`, kernel 7.0): the envelope gains helper, stage, package and kernels-without-module fields, and a fact that does not exist is `null`, not `false`: when the repair is refused up front (broken helper, dpkg not answering, no headers for the kernel) `module_loaded`/`service_active`/`rc` are `null` and the reason is in `error`; on the helper path `service_active:null` when the module did not load. `restore` checks the backup before stopping the service (a refusal is the old `ok:false`, `rolled_back:false` envelope with no snapshot left behind), the rollback brings the files exactly to the snapshot and, with `rolled_back:true`, carries an additive `rollback_complete`. Install flag `--client-ipv6-direct` (`CLIENT_IPV6_DIRECT=1`): the mode 2 list has no IPv6 part, `regen` strips `2000::/3`/`::/0` from a client whose IPv4 part equals the server list. `--jc=0`, `sbin` in PATH, backup and `restore` of generation 3.1, on 3.1 `add`/`regen` refuse without `qrencode`/`perl` | Before awgram v0.12.3 the `null` broke parsing of the `repair-module` reply ("failed to parse server response"); now a refusal is an operation error with the reason in the bot log, and `service_active:null` with `rc:1` is the usual "module failed to load". `rollback_complete:false` gets its own incomplete-rollback warning instead of "configuration was rolled back". With `CLIENT_IPV6_DIRECT=1` the "exclude from VPN" mode still sends `2000::/3`: the installer leaves per-client routes alone, so such a client's IPv6 goes through the tunnel; if `regen` stripped the IPv6 part, the routes screen shows the list as set manually. `add`/`regen` refusals are the old `status:"error"`; `--jc=0` does not affect `check --json` |

## How a new installer release is verified

When the installer ships a new release, the `manage_amneziawg.sh` diff is
reviewed: `--json` envelopes of the subcommands in use, exit codes, new
fields and warnings. If the contract is intact, only this table and the
README badge are updated; if it changed, the fix lands in the CHANGELOG as
compatibility with that specific version.
