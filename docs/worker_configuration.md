# Worker Configuration

When configuring the worker, cli arguments will always take over the file configuration if provided.

## Options

```text
Usage: aur-build-worker [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>
          Path of the server configuration file. Default: './config_worker.json'
      --log-level <LOG_LEVEL>
          Log level. Default: 'info' [possible values: off, error, warn, info, debug, trace]
      --log-path <LOG_PATH>
          Log file output for the worker. Default: './aur-build-worker.log'
  -p, --pacman-config-path <PACMAN_CONFIG_PATH>
          Path to the pacman configuration to use. Default: './config/pacman.conf'
  -m, --pacman-mirrorlist-path <PACMAN_MIRRORLIST_PATH>
          Path to the pacman mirrorlist to use. Default: './config/mirrorlist'
  -d, --data-path <DATA_PATH>
          Path to the directory where packages will be cloned and built. Default: './worker/data'
  -s, --sandbox-path <SANDBOX_PATH>
          Path to the directory where the sandbox will be stored. Default: './worker/sandbox'
  -l, --build-logs-path <BUILD_LOGS_PATH>
          Path to the directory where build logs will be stored. Default: './worker/logs'
  -b, --base-url <BASE_URL>
          Base url to the server. Example: 'http://server:8888'
  -w, --base-url-ws <BASE_URL_WS>
          Base websocket url to the server. Example: 'ws://server:8888'
  -k, --api-key <API_KEY>
          API key to use for authentication
  -f, --force-base-sandbox-create <FORCE_BASE_SANDBOX_CREATE>
          Should the worker rebuild its sandbox from scratch at startup. Default 'false' [possible values: true, false]
  -h, --help
          Print help
  -V, --version
          Print version
```

## File

See a full example with default values in `config_worker.json.sample`.

| Key                         | Required | Default                  | Description                                                                  |
|-----------------------------|----------|--------------------------|------------------------------------------------------------------------------|
| `base_url`                  | no       | None                     | Base url to the server                                                       |
| `base_url_ws`               | no       | None                     | Base websocket url to the server.                                            |
| `api_key`                   | yes      | None                     | API Key to use to authenticate the to server                                 |
| `data_path`                 | no       | `./worker/data`          | Path to the directory where packages will be cloned and built                |
| `sandbox_path`              | no       | `./worker/sandbox`       | Path to the directory where the sandbox will be stored                       |
| `build_logs_path`           | no       | `./worker/logs`          | Path to the directory where build logs will be stored                        |
| `pacman_config_path`        | no       | `./config/pacman.conf`   | Path to the pacman configuration to use.                                     |
| `pacman_mirrorlist_path`    | no       | `./config/mirrorlist`    | Path to the pacman mirrorlist to use                                         |
| `log_path`                  | no       | `./aur_build_server.log` | Log file for the app.                                                        |
| `log_level`                 | no       | `info`                   | Log level for the app. possible values: off, error, warn, info, debug, trace |
| `force_base_sandbox_create` | no       | `false`                  | Set to `true` if you want the worker to recreate the base sandbox at start   |