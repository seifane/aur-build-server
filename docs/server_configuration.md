# Server Configuration

When configuring the server, cli arguments will always take over the file configuration if provided.

## Command Line Options

```text
Usage: aur-build-server [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>
          Path of the server configuration file. Default: './config_server.json'
      --log-level <LOG_LEVEL>
          Log level. Default: 'info' [possible values: off, error, warn, info, debug, trace]
      --log-path <LOG_PATH>
          Log file output for the server. Default: './aur-build-server.log'
  -k, --api-key <API_KEY>
          Sets the API Key that will be used by the workers and CLI to authenticate
  -p, --port <PORT>
          Port to listen on. Default: '8888'
  -r, --repo-name <REPO_NAME>
          Name of the Arch repo to create and serve
  -s, --sign-key <SIGN_KEY>
          ID of the GPG key used to sign the packages
  -t, --rebuild-time <REBUILD_TIME>
          The time in seconds between rebuild attempts
      --serve-path <SERVE_PATH>
          Path to store built packages and serve them. Default: './server/serve'
      --build-logs-path <BUILD_LOGS_PATH>
          Path to store built packages and serve them. Default: './server/build_logs'
  -d, --database-path <DATABASE_PATH>
          Path to store database. Default: './server/aur-build.sqlite'
      --webhook-verify-ssl <WEBHOOK_VERIFY_SSL>
          Verify the validity of the presented ssl certificate. Default: 'true' [possible values: true, false]
      --webhook-certificate <WEBHOOK_CERTIFICATE>
          Trust this certificate when sending webhooks. Must be a path to a valid .pem certificate
  -h, --help
          Print help
  -V, --version
          Print version
```

## Configuration File

See a full example with default values in `config_server.json.sample`.

| Key                   | Required | Default                     | Description                                                                                                                           |
|-----------------------|----------|-----------------------------|---------------------------------------------------------------------------------------------------------------------------------------|
| `log_level`           | no       | `info`                      | Log level for the app. possible values: off, error, warn, info, debug, trace                                                          |
| `log_path`            | no       | `./aur_build_server.log`    | Log file for the app.                                                                                                                 |
| `api_key`             | yes      | None                        | API Key that will be used by the workers and CLI to authenticate                                                                      |
| `port`                | no       | `8888`                      | Port to listen on.                                                                                                                    |
| `repo_name`           | no       | `aurbuild`                  | Name of the Arch repo to create and serve                                                                                             |
| `sign_key`            | no       | None                        | The GPG key to use to sign the packages. If none given the packages will not be signed. The given key must not have a passphrase set. |
| `rebuild_time`        | no       | None                        | The time in seconds between package rebuilds. If none are given the packages will not be rebuilt automatically.                       |
| `serve_path`          | no       | `./server/serve`            | The path were built packages, signatures and the repo files will be stored.                                                           |
| `build_logs_path`     | no       | `./server/build_logs`       | The path were logs of the builds sent back by the workers will be stored.                                                             |
| `database_path`       | no       | `./server/aur_build.sqlite` | The path to the package database.                                                                                                     |
| `webhooks`            | no       | None                        | Array of URL for webhooks. See webhooks in the docs.                                                                                  |
| `webhook_verify_ssl`  | no       | `true`                      | Enable / disable SSL certificate verification when sending webhooks.                                                                  |
| `webhook_certificate` | no       | None                        | Add an SSL certificate to trust when sending webhooks. Must be a path to a valid .pem certificate                                     |
