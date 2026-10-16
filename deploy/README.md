## Развёртывание

Сервер слушает `127.0.0.1:8090` и не имеет собственного TLS — снаружи его
закрывает обратный прокси. Примеры ниже для systemd + Caddy, но подойдёт любой
прокси, умеющий WebSocket.

### 1. Сборка

```sh
cargo build --release
```

Адрес прослушивания задаётся через `TETRIS_ADDR` (по умолчанию `127.0.0.1:8090`).

### 2. Размещение

Положить репозиторий (или только собранный бинарник) в каталог, из которого
сервису разрешено работать, например `/opt/tetris`:

```sh
sudo mkdir -p /opt/tetris
sudo cp -r . /opt/tetris/
```

Пути в `tetris.service` захардкожены на `/opt/tetris` — если выбрал другой
каталог, поправь `WorkingDirectory` и `ExecStart`.

### 3. Служба

```sh
sudo cp deploy/tetris.service /etc/systemd/system/tetris.service
sudo systemctl daemon-reload
sudo systemctl enable --now tetris
systemctl status tetris
```

### 4. Обратный прокси

Скопировать блок из `Caddyfile.snippet` в свой Caddyfile, заменить домен на свой
и перезагрузить:

```sh
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

Caddy сам получит сертификат Let's Encrypt. WebSocket проксируется автоматически,
отдельных настроек не нужно. Для nginx понадобится явно пробросить заголовки
`Upgrade` и `Connection`.

### Проверка

```sh
curl -s -o /dev/null -w '%{http_code}\n' https://<твой-домен>/
```

Ответ `200` означает, что страница отдаётся. Если страница открывается, но игра
не реагирует — проблема с WebSocket-апгрейдом на прокси.
