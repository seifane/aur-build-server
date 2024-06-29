# AUR Build Server

## Goal

This project aims to provide an external package building server based on any PKGBUILD based project.
Right now, it pulls AUR packages, builds them with makepkg and serves them on a custom Arch repo.

## Building

```bash
cargo build
```
Should build all binaries `aur-build-server`, `aur-build-worker`, `aur-build-cli`.

# Running

The project is split into two main parts. 

## Server
The server is in charge of packages, distribution of the repository and dispatching them to workers.
It loads the configuration from `config_server.json` by default.

The server will dispatch packages to be built to connected workers and receive the end product (built package + logs) to add to the repository.

## Worker
The worker connects to the server and await instructions to build packages. It is strictly in charge of building of the packages.
It loads the configuration from `config_worker.json` by default.

Once it is done with the build it will upload the artifacts back to the server.
You can have any numbers of workers at any time connected to the server. This allows to scale the number available workers based on the size of the repo you're building.

Because the program uses pacman to install dependencies it needs root. However running it directly in root is a **bad** idea.
The better way would be to run it with a user that has a no password root authorized access to pacman.
Right now there's a Dockerfile that does that.


## Docker 
You can use the provider docker compose setup to start the project.

```bash
cp config_server.json.sample config_server.json
cp config_worker.json.sample config_worker.json
docker-compose up -d --build
```

# Documentation

## Server

```text
Usage: aur-build-server [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>  [default: config_server.json]
      --log-level <LOG_LEVEL>      [default: info] [possible values: off, error, warn, info, debug, trace]
  -l, --log-path <LOG_PATH>        [default: aur-build-server.log]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Worker

```text
Usage: aur-build-worker [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>  [default: config_worker.json]
      --log-level <LOG_LEVEL>      [default: info] [possible values: off, error, warn, info, debug, trace]
  -l, --log-path <LOG_PATH>        [default: aur-build-worker.log]
  -h, --help                       Print help
  -V, --version                    Print version

```

## CLI

A CLI is provided to interface with the server. It currently allows to fetch the packages and their current status as well as logs.

```text
Usage: aur-build-cli [OPTIONS] <COMMAND>

Commands:
  packages  Packages related commands. list, rebuild
  logs      <package> Fetch the logs for the given package
  profiles  Profile related commands. list, create, delete, set-default
  help      Print this message or the help of the given subcommand(s)

Options:
      --base-url <BASE_URL>  Base url of the server. Will take over the profile if specified along with api-key
      --api-key <API_KEY>    Api key of the server. Will take over the profile if specified along with base-url
  -p, --profile <PROFILE>    Profile name to use
  -h, --help                 Print help
  -V, --version              Print version
```

## API

- `GET /repo` Exposes the created Arch repository
- `GET /api/workers` Returns a list of currently connected workers and their status
- `GET /api/packages` Json response including current state of build of all packages
- `POST /api/rebuild` Queues the specified packages for a rebuild, if no specified packages queues all packages
  - ```json
    { "packages": ["platform"] }
    ```
- `GET /api/logs/:package_name` Get build logs for a given `package_name`.
- `POST /api/webhooks/trigger/package_updated/:package_name` Manually trigger a PackageUpdated webhook for the given package name.

### API Authentication
The API are protected using an API key specified in the `config_server.json` file.
You can authenticate a request by including the API key in the `Authorization` header.

## Configuration

### Server

- `repo_name` (required) : The name of the repo that will be used for repo-add.
- `sign_key` (optional, default: null) : ID of the GPG key that the server should use when trying to sign the packages and the repo.
- `api_key` (required) : The api key that will be used to authenticate workers and api consumers.
- `rebuild_time` (optional, default: null) : The amount of seconds to wait before trying to rebuild a package. If null is given packages will not be regularly rebuilt.
- `packages` (required) : The packages that should be built.
  - `name` (required) : Defines the name of the package from aur.
  - `run_before` (optional) : Defines a bash command to run before trying to build the package (see sample config).
  - `patches` (optional) : Defines a list of patches to be applied to the downloaded package files
    - `url` (required) : The url of the patch
    - `sha512` (optional) : If given this is the SHA512 checksum for the patch file
- `serve_path` (optional, default: `serve/`) : Path to the directory which will contain the package files, it will also be served by the http server at `/repo`
- `port` (default: 8888) : Port that the server will listen on.
- `webhooks` (optional) : Array of URLs that will get sent webhooks on events related to packages. See the webhooks section.

#### Signing

To set up the packing signing you can make use of the `SIGN_KEY_PATH` environment variable in the docker container for the server.
In the server configuration fill in the key ID and the server will try to sign built packages and repo database.

### Worker

- `base_url` (required) : The base url of the server
- `base_url_ws` (required) : The base url websocket of the server
- `api_key` (required) : The api key used to authenticate on the server

# Pacman configuration

To add the custom repository to your pacman configuration you can simply append the following section to your `/etc/pacman.conf` file.

```text
[aurbuild]
Server = http://your-server-domain-or-ip/repo
```

Make sure to replace `aurbuild` with the name you put in the server configuration under `repo_name`.

If you do not enable signing you will need to add the following line to disable signature checking.
```text
SigLevel = Optional TrustAll
```

# Webhooks

You can specify a list of URLs that will get POST'ed a payload when events related to packages happen.
All webhooks are POST to the URL with a `type` corresponding to the event being trigger and the `payload` for the given type.

## Webhook events

- PackageUpdated : Triggers when the package is scheduled for rebuild, has been built with a new version or failed to build.

Example:
```json
{
  "type": "PackageUpdated",
  "payload": {
    "package": {
      "name": "google-chrome",
      "run_before": null,
      "patches": null
    },
    "status": "BUILT",
    "last_built": "2024-01-01T00:00:00",
    "last_built_version": "1.2.3",
    "last_error": null
  }
}
```

# Roadmap
- [ ] Add package version history, ability to keep some amount of version of the same package and keep serving them.
- [ ] Support repos from custom sources that are not aur (git, ...) 
