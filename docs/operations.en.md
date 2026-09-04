# Operations: updates and backups

[Русская версия](operations.md)

Operator notes that did not fit the README: update channels, backup
caveats in hardened mode, and manual restore without the bot.

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
