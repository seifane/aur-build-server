# AUR Build Server

## Goal

This project aims to provide an external package making server based on any PKGBUILD based project.
Right now it pulls AUR packages builds them with makepkg and serves them on a custom Arch repo.

## Api

- `GET /repo` Exposes the created Arch repository
- `GET /api/packages` Json response including current state of build of all packages
- `GET /api/packages/rebuild` Launches a rebuild for all the packages
- `GET /api/packages/rebuild/{package_name}` Launches a rebuild for the specified package
- `GET /api/start` Starts the workers to process packages
- `GET /api/stop` Stops the workers processing packages
- `GET /api/commit` Pull built packages that where not yet included in the repository

### Api Authentication
The API can be protected using an API key specified in the `config.json` file.
You can auth a request by including the API key in the `Authorization` header or with `?apikey=` as a query string.

# TODO
- [ ] Some stuff is still hardcoded (like repo name)
- [ ] Include CRON-like system to try to rebuild package regularly
- [ ] Handle command line arguments in docker image
- [ ] More documentation on cmd args
- [ ] Currently, packages are only rebuilt when the there's a new commit on the cloned AUR repository.
  That may not be the best method for all AUR packages.
  Include a way to force some packages to always be rebuilt.
- [ ] Probably better api than only GET routes ?
- [ ] Make use of a proper logging library