# CLI

A CLI is provided to interface with the server. It currently allows to fetch the packages and their current status as well as logs.

```text
Usage: aur-build-cli [OPTIONS] <COMMAND>

Commands:
  workers   Get the list of current workers
  packages  Packages related commands. list, rebuild
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