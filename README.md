# Music Cloud CLI

Установщик и административный CLI Music Cloud.

```bash
cargo install --path . --locked
mc-cli --help

# установка с приватным GHCR
GHCR_TOKEN=... mc-cli --dir /opt/music-cloud install \
  --library /srv/music-cloud/library \
  --ghcr-user YOUR_GITHUB_USER \
  --profiles worker,transcode

# обновление до нового тега
GHCR_TOKEN=... mc-cli --dir /opt/music-cloud update \
  --tag v0.1.1 --ghcr-user YOUR_GITHUB_USER
```

В репозитории лежат копии `compose.yml`, `.env.example` и monitoring-конфигов из `moshms/deploy` — они встраиваются в бинарь для `mc-cli install`. При изменении deploy-файлов синхронизируйте `assets/`.

CI: fmt/clippy/test на всех ветках. На тегах собирается статический musl-бинарь и прикрепляется к GitHub Release.
