# Running locally

## Setup

Make sure you have installed the dependencies given in the main [readme](../README.md).
You should not be running the server or the workers as root.

Make sure your configuration files are created. If not you can run the following command to get the basic configuration created.
The sample config files are targeting docker by default so you will need to adjust `base_url` and `base_url_ws` to point to your server. Most likely this will be `http://localhost:8888` and `ws://localhost:8888` respectively.

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

Start the server

```bash
cargo run --bin aur-build-server
```

Start the worker

```bash
cargo run --bin aur-build-worker
```

### Running multiple workers

To run multiple workers on the same machine you will need to set different `data_path`, `sandbox_path` and `build_logs_path` for each instance of the worker.
For more information check the [worker configuration](worker_configuration.md)