# Server Configuration

When configuring the server, cli arguments will always take over the file configuration if provided.

## Options

```text
Usage: aur-build-server [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>
          Path the server configuration file. Default: './config_server.json'
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
  -h, --help
          Print help
  -V, --version
          Print version
```

## File

See a full example with default values in `config_server.json.sample`.

| Key               | Required | Default                  | Description                                                                                                                           |
|-------------------|----------|--------------------------|---------------------------------------------------------------------------------------------------------------------------------------|
| `log_level`       | no       | `info`                   | Log level for the app. possible values: off, error, warn, info, debug, trace                                                          |
| `log_path`        | no       | `./aur_build_server.log` | Log file for the app.                                                                                                                 |
| `api_key`         | yes      | None                     | API Key that will be used by the workers and CLI to authenticate                                                                      |
| `port`            | no       | `8888`                   | Port to listen on.                                                                                                                    |
| `repo_name`       | no       | `aurbuild`               | Name of the Arch repo to create and serve                                                                                             |
| `sign_key`        | no       | None                     | The GPG key to use to sign the packages. If none given the packages will not be signed. The given key must not have a passphrase set. |
| `rebuild_time`    | no       | None                     | The time in seconds between package rebuilds. If none are given the packages will not be rebuilt automatically.                       |
| `serve_path`      | no       | `./server/serve`         | The path were built packages, signatures and the repo files will be stored.                                                           |
| `build_logs_path` | no       | `./server/build_logs`    | The path were logs of the builds sent back by the workers will be stored.                                                             |
| `webhooks`        | no       | None                     | Array of URL for webhooks. See webhooks in the docs.                                                                                  |
| `packages`        | yes      | None                     | Array of package definition for the packages to build. See the package section.                                                       |


### Package configuration

| Key          | Required | Default | Description                                                                                                                                                                  |
|--------------|----------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `name`       | yes      | N/A     | The name of the AUR package to build                                                                                                                                         |
| `run_before` | no       | None    | A command that will run before the package build process. This is useful to add gpg keys needed by the packages or installing some dependencies manually. See sample config. |
| `patches`    | no       | None    | Array of patches to be applied to the cloned package                                                                                                                         |


#### Patches configuration

| Key      | Required | Default | Description                                                                                         |
|----------|----------|---------|-----------------------------------------------------------------------------------------------------|
| `url`    | yes      | None    | URL to the git patch.                                                                               |
| `sha512` | no       | None    | SHA512 hash of the git patch. If provided the patch will not be applied if the hash does not match. |
