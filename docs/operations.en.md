# Operations: updates, backups and uninstall

[Русская версия](operations.md)

Operator notes that did not fit the README: update channels, backup
caveats in hardened mode, manual restore without the bot, and uninstall.

## Update channels

`awgram-setup update` installs the latest stable release. Pre-release
builds (`vX.Y.Z-rc.N`) are available since **v0.7.0**:

```bash
awgram-setup update --channel rc       # switch to pre-releases
awgram-setup update --channel stable   # back to stable
```

The channel choice is sticky; the `rc` channel also sees stable releases,
so once the final version ships the update arrives on its own.

If the server's `awgram-setup` is older than v0.7.0, it has no `--channel`
flag yet. Options:

- run a plain `awgram-setup update` — it updates the script itself too,
  after which `--channel` becomes available;
- install an rc right away with a one-liner from the release:

  ```bash
  curl -fsSL https://github.com/ekuraev/awgram/releases/download/vX.Y.Z-rc.N/install.sh \
    | bash -s -- update --channel rc
  ```

## Backups in hardened mode

The installer creates its archives with `chmod 600`, unreadable by the
`awgram` user. Backups through the bot therefore fully work in **root**
mode. In hardened mode grant read access manually:

```bash
setfacl -m u:awgram:r /root/awg/backups/*.tar.gz
```

or wait for an installer update
([bivlked/amneziawg-installer#256](https://github.com/bivlked/amneziawg-installer/issues/256)).

## Manual restore from a bundle

The bot stores backups in `backups/awgram/` as `awgram_backup_<ts>.tar.gz`:
inside are the installer archive byte for byte, `meta.json` and, if
enabled, a snapshot of the bot's database. If the bot itself is
unavailable, AmneziaWG can be restored with the stock script:

```bash
tar -xzf awgram_backup_<ts>.tar.gz awg/ \
  && sudo bash /root/awg/manage_amneziawg.sh restore awg/awg_backup_<ts>.tar.gz
```

The bot's database (`awgram.db` inside the bundle) is not touched by this
restore — put it back manually while the `awgram` service is stopped.

## Uninstall

```bash
sudo awgram-setup uninstall
```

The command asks for confirmation and removes only the bot itself:

| Removed | Kept |
|---|---|
| the `awgram` service (stopped, disabled, unit file deleted) | AmneziaWG, `awg0`, the clients and their configs |
| the binary `/usr/local/bin/awgram` and its `.bak` copy | the bot's backups in `<clients dir>/backups/awgram` |
| the `/etc/sudoers.d/awgram` rule and the `awgram` user (hardened) | the `awgram` user's ACLs on the clients dir (hardened) |
| `awgram-setup` itself | the AmneziaWG installer and its scripts |

The config, the token and the bot's data — `/etc/awgram` (`config.toml`,
`env` with the token, `setup.conf`, and `state.json` in root mode) and
`/var/lib/awgram` (`awgram.db`: groups, history, settings) — are removed
separately:

| Invocation | Config and data |
|---|---|
| `awgram-setup uninstall` in a terminal | a second question: remove or keep |
| `awgram-setup uninstall --purge` | removed without asking |
| `awgram-setup uninstall --yes` | kept |
| `awgram-setup uninstall --yes --purge` | removed; no questions at all |

A later install picks up the config and data that were kept: it takes the
token and `admin_ids` from them without asking again, and the groups and
history come back with `awgram.db`. The token stays on disk in
`/etc/awgram/env` meanwhile — if the bot is gone for good, uninstall with
`--purge` and revoke the token at @BotFather.

Without a terminal (cron, CI, `ssh` with no tty) `--yes` is required:
otherwise there is nothing to confirm the removal with, and the command
exits without removing anything.

If `awgram-setup` is already gone from the server, the installer from the
release runs the same command:

```bash
curl -fsSL https://github.com/ekuraev/awgram/releases/latest/download/install.sh \
  | sudo bash -s -- uninstall
```

What is left to clean up by hand if everything has to go:

```bash
# the bot's backups (the path is your clients dir, /root/awg by default)
sudo rm -rf /root/awg/backups/awgram

# hardened mode: the awgram user's ACLs on the clients dir.
# Run this BEFORE uninstall, while the user still exists:
sudo setfacl -R -x u:awgram /root/awg
sudo setfacl -R -d -x u:awgram /root/awg
```

Help for the command — `awgram-setup help uninstall`; the other commands
work the same way: `awgram-setup help install | update | config | status`
or `awgram-setup <command> --help`.
