# AUR Build Server

## Goal

This project aims to provide an external package building server based on any PKGBUILD based project.
Right now it pulls AUR packages builds them with makepkg and serves them on a custom Arch repo.

## Building

```bash
cargo build
```

or Docker image:

```bash
docker build -t aur-build-server:latest .
```

## Running

The project is split into two main parts. 

### Server
The server is in charge of packages, distribution of the repository and dispatching them to workers.
It loads the configuration from `config_server.json` by default.

The server will dispatch packages to be built to connected workers and receive the end product (built package + logs) to add to the repository.

### Worker
The worker connects to the server and await instructions to build packages. It is strictly in charge of building of the packages.
Once it is done with the build it will upload the artifacts back to the server.
You can have any numbers of workers at any time connected to the server. This allows to scale the number available workers based on the size of the repo you're building.

Because the program uses pacman to install dependencies it needs root. However running it directly in root is a **bad** idea.
The better way would be to run it with a user that has a no password root authorized access to pacman.
Right now there's a Dockerfile that does that.


### Docker 
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

## Api

- `GET /repo` Exposes the created Arch repository
- `GET /api/workers` Returns a list of currently connected workers and their status
- `GET /api/packages` Json response including current state of build of all packages
- `POST /api/rebuild` Queues the specified packages for a rebuild, if no specified packages queues all packages
  - ```json
    { "packages": ["platform"] }
    ```
- `GET /api/logs/:package_name/:suffix` Get build logs for a given `package_name`. Valid options for suffix are
  - `stdout` : stdout output of makepkg
  - `stderr` : stderr output of makepkg 
  - `stdout_before` : stdout output of run_before command if any
  - `stderr_before` : stderr output of run_before command if any
  - `stdout_deps` : stdout output of pacman install of dependencies
  - `stderr_deps` : stderr output of pacman install of dependencies

### Api Authentication
The API are protected using an API key specified in the `config_server.json` file.
You can authenticate a request by including the API key in the `Authorization` header.

## Configuration

## Server

- `repo_name` (required) : The name of the repo that will be used for repo-add.
- `sign` (default: false) : If true, the server will try to sign the packages and the repo using gpg.
- `api_key` (required) : The api key that will be used to authenticate workers and api consumers.
- `rebuild_time` (optional) : The amount of seconds to wait before trying to rebuild a package. If none is given packages will not be automatically rebuilt.
- `packages` (required) : The packages that should be built.
  - `name` (required) : Defines the name of the package from aur.
  - `run_before` (optional) : Defines a bash command to run before trying to build the package (see sample config).
- `port` (default: 8888) : Port that the server will listen on.

## Worker

- `base_url` (required) : The base url of the server
- `base_url_ws` (required) : The base url websocket of the server
- `api_key` (required) : The api key used to authenticate on the server

# Roadmap
- [ ] Support applying patches on repos
- [ ] Support repos from custom sources that are not aur (git, ...) 
