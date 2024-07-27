# Change log

## 0.20.0

### Breaking changes

- Default directories for the worker and the server were moved into their own subfolders. Check the new default configs for the paths or adjust your setup accordingly.
- The worker now uses bubblewrap to create sandboxes to build packages. This means that the docker containers for the worker needs more privileges. This is explained the docker docs.
- Configuration have been updated for the server / worker.

### Changes
- The worker now tries to fetch GPG keys declared in the makepkg.
- When building packages the worker now builds a dependency graph first for aur dependencies. This should improve successful build rates.
- Worker will try to reconnect to the server if the connection is lost
- Build log files are now unified. The logs for all aur dependencies for a packages will be added to the same log file.

Other changes
- Heavy refactoring on the worker

## 0.11.0

### Breaking changes
- Some fixes were made on the handling of state restoration of the repository on start up.
  It is advised you clear the serve folder before upgrading to prevent any lingering issues. You can also try to force a rebuild on all packages.

### Changes

- Ability to use webhooks to notify when a package has been updated
- Ability to evict workers

Other changes
- Rewrite of server for future features
- Started implementing testing

## 0.10.0

### Breaking changes

- The handling of log files has been updated. Now all logs are merged into one file for each declared package. 
This means that the REST API has been updated to reflect this.

The route to get logs has been updated from `GET /api/logs/:package_name/:suffix` to `GET /api/logs/:package_name`

- The configuration for the signing was updated. Before, the signing was enabled via the boolean `sign` and the server was using the default GPG key.

`sign` is now ignored.
The server configuration now takes a new key `sign_key`. This should be the ID of the key to be used when signing. If no `sign_key` is given signing will be disabled.

### Changes

- You can now apply git patches to the fetched repository (the actual AUR repo being cloned).
See the README and sample config for documentation on the usage.


