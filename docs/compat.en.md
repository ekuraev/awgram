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
| Supported (`--json` contract verified) | [v6.3.10](https://github.com/bivlked/amneziawg-installer/releases/tag/v6.3.10) |
| Minimum | [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0) |

v5.20.x and older are not supported: the bot relies on the extended
`--json` interface for management commands introduced in v5.21.0.

Subcommands used, all with `--json`: `add`, `remove`, `list`, `stats`,
`regen`, `modify`, `backup`, `restore`, `check`, `restart`, `repair-module`.
Plus `diagnose`, as plain text without `--json`: the bot shows its report as is.

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

## The server address outside the client's AllowedIPs

Since v6.0.0 `add` and `regen` carve the IPv4 address from the client's
`Endpoint` out of its AllowedIPs: a `/n` route covering the address becomes
`32−n` routes (`0.0.0.0/0` is left alone; a hostname or IPv6 gets no hole).
This is a fix for Linux clients on `awg-quick`: it installs its routing rule
only for `0.0.0.0/0`, so a list covering the server's address sent the
packets to the server itself into the tunnel. The Windows, Android and iOS
apps add a route to the server themselves, the hole does not affect them.
The carve has no switch.

For the bot this means:

- the `allowed_ips` field of the `add --allowed-ips` reply is wider than the
  list passed in: in the "exclude from VPN" mode the server's public address
  is always inside;
- the routes screen matches the client's list against the presets while
  tolerating a hole at the IPv4 from the same `.conf`'s `Endpoint`: such a
  client shows as its preset, not as "set manually"; a hole at any other
  address still counts as a manual list;
- the installer's `modify AllowedIPs` writes the list as given (it only
  warns), so on "Apply" the routes screen carves the server address itself,
  by the same rule: the config after an edit through the bot matches what
  `add`/`regen` would issue, and the hole does not vanish until the next
  `regen`. The bot does this on any installer version: on older ones the
  hole is harmless (the apps add a route to the server themselves), and
  their `regen` treats such a list as individual and leaves it alone. If
  the only IPv4 route left would be the server address, the bot leaves the
  list unchanged — `modify` writes it as given and warns;
- a list whose only IPv4 route is the server address is refused by
  `add`/`regen` before the config is written — with the old
  `status:"error"`; the bot's presets never build such a list.

## PresharedKey by default and `add --no-psk`

Since v6.3.0 a plain `add` gives the new client a PresharedKey; a client
without a PSK is created with `add --no-psk`. On a server installed with
`--no-psk` (`NO_PSK=1` in `awgsetup_cfg.init`) it is the other way round:
no PSK without flags, and `--psk` turns it on. Before v6.3.0 only `--psk`
gave a PSK.

The bot asks about the PSK when a client is created (the "With PSK" /
"No PSK" buttons, ordered by the "Default PSK" setting), so it passes the
choice explicitly both ways and does not depend on the server's setting:

- "With PSK" — `add --psk`, as before, on any installer version;
- "No PSK" — `add --no-psk` when the script's help lists the flag (`--help`,
  the same detection as for `--allowed-ips`); an installer older than v6.3.0
  has no such flag and the bot does not send it — a client without `--psk`
  has no PSK there anyway;
- the help promised `--no-psk` but the script rejected it (the emergency
  envelope with `command:"help"`, no client created) — the bot remembers
  that and repeats `add` without the flag.

awgram before v0.13.4 does not know `--no-psk`: on installer v6.3.0 and
newer a client created with the "No PSK" button gets a PresharedKey anyway.
The connection is not affected — the key lands in both the server config
and the client config.

Clients already issued are untouched by the new default, and `regen` keeps
whatever a client has. `--psk` and `--no-psk` together, or with a command
other than `add`, are now an error; the bot sends at most one of them and
only with `add`.

## Installer release history

Releases v5.21.1–v5.37.0 did not break the JSON contract: all new messages
went to stderr and the `--json` envelopes on stdout were unchanged. v5.37.1
is the first where a field of an existing envelope changed its type (`null`
instead of `false` in `repair-module --json`), see its row below. v6.0.0 did
not change the envelopes but changed the contents of `allowed_ips` and of
the client `.conf` files (the server-address carve), see its row and the
section above.
v6.0.1–v6.1.1 left the envelopes and the profiles alone: the new `Jc`
warnings go to stderr, see their row below.
v6.1.2 added a CVE-2026-98432 guard report to `check` and a `qr` command:
the envelopes are the same, the report goes to stderr, see its row below.
v6.2.0–v6.2.2 moved new ARM64 installs to AmneziaWG 3.1 and fixed the hints
for a prebuilt ARM module after a kernel change: the envelopes are the same,
the hints go to stderr, see their row below.
v6.3.0–v6.3.1 left the envelopes of the existing commands alone but changed
the default of `add`: a new client gets a PresharedKey unless `--no-psk` is
passed — see their row and the section above.
v6.3.2–v6.3.10 did not break the envelopes of the commands in use: the port
in `check --json` gained the fields `source` and `init_mismatch`, and the
port itself is now taken from `ListenPort` in the live `awg0.conf`; `regen`
on 2.0 deletes the old QR and `vpn://` before rewriting `.conf`; the
`block-ru` command was added, which the bot does not call — see their row
below.

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
| v6.0.0 | A fresh install sets up AmneziaWG 3.1 by default (`HeaderProtectionKey`, `ContentPaddingAddition`), 2.0 where 3.1 cannot run; working servers stay on their generation, and a server still has one generation and one `awg0` interface. `add`/`regen` carve the server's IPv4 from `Endpoint` out of the client's AllowedIPs and refuse a list whose only IPv4 route is that address; `modify` warns about the server address in the list and about a new `Endpoint` under the client's routes. `check`/`diagnose` warn when a data packet matches a handshake packet in length and when the kernel module is reachable without root (verdict and JSON unchanged). `manage` 6.0.0 with an older `awg_common.sh` refuses with the old `ok:false` envelope. Fields, exit codes and `ok` of the `--json` envelopes are the same as in v5.37.1 | The routes screen tolerates the hole at the server address and carves it itself on an edit through `modify` (see the section above); before awgram v0.13.0 a client with the hole showed as "set manually" with empty toggles. The "Server check" card shows the generation from `protocol` (`2.0`/`3.1`, the field exists since v5.36.0) and warns when the marker is unreadable; on an installer without the field there is no line. The installer does not run a second interface for 2.0 and 3.1, so a per-client generation choice does not appear in the bot. `add`/`regen` refusals and a mismatch of the installer halves come as the old envelopes; the new `check` warnings go to stderr |
| v6.0.1–v6.1.1 | A warning about `Jc` beyond the amneziawg-go batch: a client on amneziawg-go sends the junk packets, the non-empty `I1`–`I5` and the handshake in one batch of 128 packets and crashes on the first handshake when it overflows (amneziawg-go#111); `check`/`diagnose`, `add`/`regen`, `modify` (when it rebuilds the QR and `vpn://`) and the config validator warn when `Jc + number of I + 1 > 128` and say how to lower `Jc` (the `check` verdict, the envelopes and the profiles do not change); the install flag `--jc` is limited to 0–122. The install flag `--i1` pins a custom `I1` packet in `awgsetup_cfg.init` (`AWG_I1_PIN`), profiles get it after `regen`. The validation summary names the config's generation (3.1 or 2.0). The rest is installer and docs: apt output and git's answer in the install log, a server DNS check before the PPA unavailability message, docs that start from the 3.1 path | None: the `Jc` warnings go to stderr, `check --json` does not carry them, the bot neither shows them nor sets `Jc`; a pinned `I1` arrives in `add`/`regen` profiles as an ordinary parameter. Fields, exit codes and `ok` of the `--json` envelopes are the same as in v6.0.0 |
| v6.1.2 | Protection against the kernel vulnerability CVE-2026-98432 (a bug in parsing a TCP SYN with IP options, critical): the installer sets a firewall rule in the `raw` table (TCP packets with IP options are dropped before the kernel parses them) and the unit `amneziawg-ipopts-guard.service`; on a running server the mode `install_amneziawg.sh --ipopts-guard` adds it; `check`/`status` report whether the guard is in place (no `amneziawg-ipopts-guard` helper, no rule, unit not enabled, status check failed) — messages only, the verdict does not change. The command `qr <name> [--vpn]` prints the config QR or the `vpn://` link QR right in the terminal ([#375](https://github.com/bivlked/amneziawg-installer/issues/375)); it refuses `--json`, and `--vpn` with any other command is an error with exit code 1. The rest is installer and docs: Ubuntu kernels 7.0.0-42/-43, the "No connection: where to start" section in ADVANCED | None: the guard report goes through `log`/`log_warn`, that is to stderr, `check --json` has no new fields, the verdict and the exit code are the same; the bot uses neither the `qr` command nor the `--vpn` flag — it still takes the QR and the link from the `.png`/`.vpnuri` files that `add`/`regen`/`modify` create. The helper check is capped by `timeout 10` and fits in the bot's operation timeout (60 s by default). Fields, exit codes and `ok` of the `--json` envelopes are the same as in v6.0.0 |
| v6.2.0–v6.2.2 | AmneziaWG 3.1 on 64-bit ARM: a new install on ARM64 gets the 3.1 profile and the PPA module through DKMS (32-bit ARM stays on 2.0, running servers keep their generation), and `restore` of a 3.1 backup on arm64 no longer refuses with the `arm` code. A server on the prebuilt ARM module package after a kernel change: instead of "normal for userspace mode", `check` warns that there is no module for the kernel, and `repair-module` on the legacy path (no helper) names what works instead of the `dkms` recipe — running the installer again, which keeps the keys and the profiles. The rest is installer and docs: apt download retries are off during an install and `--uninstall` (CVE-2026-109518), prebuilt modules for Raspberry Pi OS trixie, removal of the Oracle Cloud image's `REJECT` rule sitting above the UFW rules, the client table | None: one `check` branch changed in `manage_amneziawg.sh` — a warning through `log_warn`, that is to stderr; `module.loaded`, the verdict and the exit code of `check --json` are the same. The `repair-module` recipe goes through `log_error` (stderr), the return code is unchanged (1), the envelope is the same. A `restore` refusal already came as the `ok:false`, `rolled_back:false` envelope; on arm64 it is now gone. The bot still labels a module that is not loaded "ok for userspace": `check --json` cannot tell a prebuilt ARM package from userspace mode, so on such a server after a kernel change read the output of `manage check` on the server. Fields, exit codes and `ok` of the `--json` envelopes are the same as in v6.0.0 |
| v6.3.0–v6.3.1 | A PresharedKey for every new client: a plain `add` gives a PSK, a client without one is `add --no-psk` (or a server installed with `--no-psk`: `NO_PSK=1` in the init); `--psk` and `--no-psk` together or with a command other than `add` are an error. New commands `port <number>` (change the UDP port: `ListenPort`, the init, the UFW rule, every client regenerated) and `exit-ip <IPv4\|off>` (a separate exit address: a `SNAT` for the client subnet in `PostUp`/`PostDown`), each with its own `--json` envelope. `check` verifies that the exit address is on an interface and tells "a module is on disk but not loaded" (built against the headers of a same-named kernel after a 24.04 HWE to 26.04 upgrade) from userspace mode; `repair-module` rebuilds such a module through the version 4 helper and ends with an error if the rebuilt module does not load. The rest is the installer: a random port for a new install instead of `39743`, `S4` never 16, a trailing newline in `awg0.conf` after a reinstall | The bot passes the "No PSK" choice as `--no-psk` and detects the flag via `--help` (awgram v0.13.4; before that such a client got a PSK on v6.3.0+). The bot does not call `port` or `exit-ip`: after `manage port` the client configs and QR codes are regenerated, the bot hands out the new files, and the port on the check card comes from `check --json`. A missing exit address makes `check --json` return `ok:false` with no field of its own: the check card shows ❌ in the header over green lines, and `manage check` on the server names the reason. The module messages go through `log_warn`/`log_error` (stderr); `module.loaded` is the same, and the bot still labels a module that is not loaded "ok for userspace". The `repair-module --json` envelope is unchanged: a rebuilt module that fails to load comes as the same `ok:false`. Fields, exit codes and `ok` of the envelopes of the commands in use are the same as in v6.0.0 |
| v6.3.2–v6.3.10 | `check` and `diagnose` take the port from `ListenPort` in the live `awg0.conf` (from the init only when there is no `ListenPort` line) and read that line the way AmneziaWG applies it: the key in any case, the last line when there are several, no comment after `#`. In `check --json` the port gained the additive fields `source` (`"listen_port"`, `"init"` or `null`) and `init_mismatch` (the init disagrees with `ListenPort`). `ListenPort = 0` (a random port) is not an error: the port and the UFW rule are not checked; an invalid or empty `ListenPort` is `ok:false`; in `check` the warning about a mismatch with the init goes to stderr, while `diagnose` puts it into the report as `WARN`; an invalid `ListenPort` is a `FAIL` now also without ufw, and the warning about a missing rule gets a `ufw allow` hint. `manage port <current>` with a lagging init only writes the current port to the init (`init_synced:true`) and leaves the server, UFW and the clients alone. `regen`, like `modify`, deletes the old `.png`/`.vpnuri` before rewriting `.conf`: on 2.0 (and on 3.1 without a copy of the set) a file that failed to build is absent and its field in `--json` is `null`; on 3.1 an incomplete set is replaced with the old one, as before. A failed delete aborts the regeneration with `.conf` untouched (`status:"error"`); on 2.0 the files already deleted by then stay deleted until the next `regen`. `add` and `regen` hold the config lock while they build the QR and `vpn://`. New command `block-ru [status\|on\|off\|update]`: the clients are denied an exit to Russian IPv4 addresses (an ipset set, a `REJECT` rule first in `FORWARD`, the unit `awg-block-ru`, the list refreshed weekly) with its own `--json` envelope. `check` and `diagnose` no longer warn about user namespaces when the kernel runs module 3.1.20261113-02 or newer. `repair-module` installs the kernel headers with `--force-confdef --force-confold`. The rest is installer and docs: the Secure Boot hint at step 3 and the DKMS key checked against the module signature, prebuilt ARM modules and the build on kernels older than 6.7 from the 3.1 tag with the #233 fix (the profile stays 2.0), step 1 gets past dpkg's questions about config files ([#409](https://github.com/bivlked/amneziawg-installer/issues/409)), step 2 is stricter about holds, IPv6 on Debian with ifupdown, a Launchpad outage no longer changes the package source; only the wording changed, for the `restore` refusal with the `kernel` blocker and for the `exit-ip` hint about `conntrack` | None: `source` and `init_mismatch` are additive and the bot does not read them; `number` is now the port from `ListenPort` in the live `awg0.conf`, and the check card shows it as before. With `ListenPort = 0` the bot shows "❌ Port 0/udp: NOT listening" under a green header, plus "⚠️ UFW: active, port NOT allowed" when UFW is active: in `check --json` that is `number:0`, `listening:false` with `ok:true`. The Diagnostics screen shows the text of `manage diagnose` as is, so the new report lines — the port warnings with the `manage port` command, the `FAIL` for an invalid `ListenPort`, the `ufw allow` hint — are visible in the bot; the bot does not parse them. The bot parses a `null` `qr`/`vpnuri`: after `regen` only the `.conf` is sent, with a "done" message and no warning; the user learns that the QR is missing from the 🖼 button ("QR was not generated"). The old file with the old config is no longer handed out as the new one. The bot does not call `block-ru`, and its files do not get into `backup`. The lock is held a fraction of a second longer and fits in the operation timeout (60 s by default). Fields, exit codes and `ok` of the envelopes of the commands in use are the same as in v6.0.0, except the port cases described in the middle column |

## How a new installer release is verified

When the installer ships a new release, the diffs of `manage_amneziawg.sh`
and `awg_common.sh` are reviewed (the `add`/`regen` and routing logic lives
in the library, and the two are updated as a pair): `--json` envelopes of
the subcommands in use, exit codes, new fields and warnings. If the
contract is intact, only this table and the README badge are updated; if
it changed, the fix lands in the CHANGELOG as compatibility with that
specific version.
