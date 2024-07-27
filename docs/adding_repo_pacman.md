# Adding your custom repository to pacman

To be able to use the packages built by the server you can add it directly to your `pacman.conf`.

## Pacman configuration

To add the custom repository to your pacman configuration you can simply append the following section to your `/etc/pacman.conf` file.

```text
[aurbuild]
Server = http://your-server-domain-or-ip/repo
```

Make sure to replace `aurbuild` with the name you put in the server configuration under `repo_name` in the server config.

If you do not enable signing you will need to add the following line to disable signature checking.
```text
SigLevel = Optional TrustAll
```