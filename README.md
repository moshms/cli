# Music Cloud CLI

Установщик и административный CLI Music Cloud.

```bash
cargo install --path . --locked
mc-cli --help
```

В репозитории лежат копии `compose.yml`, `.env.example` и monitoring-конфигов из `moshms/deploy` — они встраиваются в бинарь для `mc-cli install`. При изменении deploy-файлов синхронизируйте `assets/`.

CI: fmt/clippy/test на всех ветках. На тегах собирается статический musl-бинарь и прикрепляется к GitHub Release.
