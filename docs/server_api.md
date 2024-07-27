# API

- `GET /repo` Exposes the created Arch repository
- `GET /api/workers` Returns a list of currently connected workers and their status
- `DELETE /api/workers/:id` Evict a work from the pool. The connection will be terminated and the worker should terminate.
- `GET /api/packages` Json response including current state of build of all packages
- `POST /api/rebuild` Queues the specified packages for a rebuild, if no specified packages queues all packages. If force is specified to true will rebuild the package even if the version is the same.
    - ```{ "packages": ["platform"], "force": false } ```
- `GET /api/logs/:package_name` Get build logs for a given `package_name`.
- `POST /api/webhooks/trigger/package_updated/:package_name` Manually trigger a PackageUpdated webhook for the given package name.

## API Authentication
The API are protected using an API key specified in the `config_server.json` file.
You can authenticate a request by including the API key in the `Authorization` header.