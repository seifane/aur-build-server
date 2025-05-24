# CLI

A CLI is provided to interface with the server. It is the main way to interact with the packages in the build list to add, remove, check their build status and logs.

## Profiles

You might want to first create a profile to configure the URL to your build server as well as the API key.

You can do so by running
```shell
aur-build-cli profiles create
```

You can create multiple profiles and switch between them with the `-p` or `--profile` option.

## Documentation

The CLI commands are fully documented. When in doubt you can just append `-h` to your command to see the possible values and arguments.

```text
Usage: aur-build-cli [OPTIONS] <COMMAND>

Commands:
  workers   Get the list of current workers
  packages  Packages related commands. list, get, add, remove, rebuild
  patches   Patch related commands. list, add, remove
  logs      <package> Fetch the logs for the given package
  webhooks  Webhooks related commands. trigger
  profiles  Profile related commands. list, create, delete, set-default
  help      Print this message or the help of the given subcommand(s)

Options:
      --base-url <BASE_URL>  Base url of the server. Will take over the profile if specified along with api-key
      --api-key <API_KEY>    Api key of the server. Will take over the profile if specified along with base-url
  -p, --profile <PROFILE>    Profile name to use
  -h, --help                 Print help
  -V, --version              Print version
```

