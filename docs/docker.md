# Docker

There is a docker compose file available.

Docker hub images are available at https://hub.docker.com/r/seifane/aur-build-server.

## Setup

Make sure your configuration files are created. If not you can run the following command to get the basic configuration created.
It should work seamlessly with docker compose. 

Remember to change the API Key if you plan to expose the server publicly.

```bash
cp config_server.json.sample config_server.json
cp config_worker.json.sample config_worker.json
```

Update the mirrorlist to your country
```bash
reflector --country Singapore --fastest 5 --latest 100 --save ./config/mirrorlist
```

## Running

Start the docker compose stack by running 

```bash
docker-compose up -d --build
```

## Custom seccomp configuration

To be able to run in docker the worker has to be given extra permissions to some syscalls. There is two main ways to deal with this :

- Give the container privileged access.

This is not the best as it gives a lot of permissions that the container does not actually need.

- Set a custom seccomp that sets finer grained access to the required syscalls.

This is what is done in the provided docker compose. This is preferred as only the required syscalls for the worker to work properly are added.

For information the added syscalls are :

- `clone`
- `clone3`
- `mount`
- `umount`
- `umount2`
- `unshare`
- `pivot_root`

The custom seccomp profile is located at `./docker/seccomp.json`