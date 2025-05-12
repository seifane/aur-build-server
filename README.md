# AUR Build Server

## Goal

This project aims to provide an external package building server based on any PKGBUILD based project.
Right now, it pulls AUR packages, builds them with makepkg and serves them on a custom Arch repo.

## Installing

Packages are available in AUR.

- [aur-build-server](https://aur.archlinux.org/packages/aur-build-server)
- [aur-build-worker](https://aur.archlinux.org/packages/aur-build-worker)
- [aur-build-cli](https://aur.archlinux.org/packages/aur-build-cli)

## Building

Install the dependencies and build with this commands

```bash
pacman -Su base-devel git libgit2 openssl-1.1 bubblewrap fakechroot
cargo build
```
Should build all binaries `aur-build-server`, `aur-build-worker`, `aur-build-cli`.

## Overview

The project is split into two main parts. This to have better isolation between the main server serving the packages and the package building logic.
This also facilitates cloud deployment in Kubernetes for example.
Spawn more workers to build more packages concurrently.

### Server
The server is in charge of packages, distribution of the repository and dispatching them to workers.
It loads the configuration from `config_server.json` by default.

The server will dispatch packages to be built to connected workers and receive the end product (built package + logs) to add to the repository.

### Worker
The worker connects to the server and await instructions to build packages. It is strictly in charge of building of the packages.
It loads the configuration from `config_worker.json` by default.

Once it is done with the build it will upload the artifacts back to the server.
You can have any numbers of workers at any time connected to the server. This allows to scale the number available workers based on the size of the repo you're building.

The worker makes use of bubblewrap to create a sandbox for building packages in a clean chroot. This also means that the worker does not need to have special sudoers access to pacman to build packages.
However, it means that the worker won't run by default in a docker container because it makes use of syscalls that are forbidden by default.
See the docker part of the docs for more info.

### CLI

The CLI is used to manage the list of packages to build as well as patches that you might want to apply. Check the [CLI docs](./docs/cli.md) for more details.

## Getting started

- Get the project running
  - The fastest way to get started is through the provided docker compose stack. See how to get started with docker [here](./docs/docker.md). 
  - You can also run the project locally. See more [here](./docs/running_locally.md). 
  - Kubernetes manifest are provided as reference. See more [here](./docs/kubernetes.md).
- After setting up the server you will need to add to your pacman config. See more [here](./docs/adding_repo_pacman.md).

## Documentation
More documentation is available in the `docs` folder.

# Roadmap
- [ ] Support repos from custom sources that are not aur (git repositories, ...)
- [ ] Add package version history, ability to keep some amount of version of the same package and keep serving them.
