# Эксплуатация: обновления и бэкапы

[English version](operations.en.md)

Операторские заметки, которые не поместились в README: каналы обновлений,
особенности бэкапов в hardened-режиме и ручное восстановление без бота.

## Каналы обновлений

`awgram-setup update` ставит последний стабильный релиз. Предрелизные
сборки (`vX.Y.Z-rc.N`) доступны начиная с **v0.7.0**:

```bash
awgram-setup update --channel rc       # перейти на предрелизы
awgram-setup update --channel stable   # вернуться на стабильные
```

Выбор канала запоминается; канал `rc` видит и стабильные релизы, так что
после выхода финальной версии обновление придёт само.

Если на сервере `awgram-setup` старше v0.7.0, флага `--channel` у него ещё
нет. Варианты:

- выполнить обычный `awgram-setup update` — он обновит и сам скрипт, после
  чего `--channel` появится;
- поставить rc сразу однострочником из нужного релиза:

  ```bash
  curl -fsSL https://github.com/ekuraev/awgram/releases/download/vX.Y.Z-rc.N/install.sh \
    | bash -s -- update --channel rc
  ```

## Бэкапы в hardened-режиме

Инсталлер создаёт архивы с `chmod 600`, и пользователю `awgram` они
недоступны. Поэтому бэкапы через бота полноценно работают в **root**-режиме.
В hardened нужно вручную открыть архивы на чтение:

```bash
setfacl -m u:awgram:r /root/awg/backups/*.tar.gz
```

либо дождаться обновления инсталлера
([bivlked/amneziawg-installer#256](https://github.com/bivlked/amneziawg-installer/issues/256)).

## Ручное восстановление из бандла

Бот хранит бэкапы в `backups/awgram/` как `awgram_backup_<ts>.tar.gz`:
внутри — архив инсталлера байт в байт, `meta.json` и, если включено,
снимок БД бота. Если сам бот недоступен, восстановить AmneziaWG можно
штатным скриптом:

```bash
tar -xzf awgram_backup_<ts>.tar.gz awg/ \
  && sudo bash /root/awg/manage_amneziawg.sh restore awg/awg_backup_<ts>.tar.gz
```

БД бота (`awgram.db` внутри бандла) при таком восстановлении не
затрагивается — её можно положить на место вручную при остановленном
сервисе `awgram`.
