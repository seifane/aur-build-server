# AUR Build Server

## Goal

This project aims to provide an external package making server based on any PKGBUILD based project.
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

Because the program uses pacman to install dependencies it needs root. However running it directly in root is a **bad** idea.
The better way would be to run it with a user that has a no password root authorized access to pacman.
Right now there's a Dockerfile that does that except a little more extreme by allowing all root with no password.

You can run a local developer env by using the provided docker-compose and then exec into it like so :

```bash
docker-compose up -d
docker-compose exec app /bin/bash
```

and run your `cargo run` inside that container

```bash
USAGE:
    aur-build-server [OPTIONS]

OPTIONS:
    -c, --config-path <CONFIG_PATH>    [default: config/config.json]
    -h, --help                         Print help information
    -l, --log-path <LOG_PATH>          [default: aur-build-server.log]
    -L, --log-level <LOG_LEVEL>        [default: debug]
    -p, --port <PORT>                  [default: 8888]
    -s, --sign                         
    -V, --version                      Print version information
```

## Api

- `GET /repo` Exposes the created Arch repository
- `GET /api/packages` Json response including current state of build of all packages
- `GET /api/packages/rebuild` Launches a rebuild for all the packages
- `GET /api/packages/rebuild/{package_name}` Launches a rebuild for the specified package
- `GET /api/start` Starts the workers to process packages
- `GET /api/stop` Stops the workers processing packages
- `GET /api/commit` Pull built packages that where not yet included in the repository.
- `GET /api/logs/:package_name/:suffix` Get build logs for a given `package_name`. Valid options for suffix are
  - `stdout` : stdout output of makepkg
  - `stderr` : stderr output of makepkg 
  - `stdout_before` : stdout output of run_before command if any
  - `stderr_before` : stderr output of run_before command if any
  - `stdout_deps` : stdout output of pacman install of dependencies
  - `stderr_deps` : stderr output of pacman install of dependencies

### Api Authentication
The API can be protected using an API key specified in the `config.json` file.
You can auth a request by including the API key in the `Authorization` header or with `?apikey=` as a query string.

# TODO
- [x] Some stuff is still hardcoded (like repo name)
- [x] Better logging of builds (stdout & stderr of last try)
- [x] Sometimes race conditions occurs when multiple makepkg processes are syncdeps, find a way to solve this
- [x] Make use of a proper logging library
- [x] Restrict sudoers more in Dockerfile
- [x] Handle command line arguments in docker image
- [x] More documentation on cmd args
- [x] Support packages that have AUR packages as deps
- [ ] Include CRON-like system to try to rebuild package regularly
- [ ] Support patching repos
- [ ] Handle config hot reloading
- [ ] Currently, packages are only rebuilt when the there's a new commit on the cloned AUR repository.
  That may not be the best method for all AUR packages.
  Include a way to force some packages to always be rebuilt.
- [ ] Probably better api than only GET routes ?
