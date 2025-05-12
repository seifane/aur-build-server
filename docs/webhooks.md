# Webhooks

You can specify a list of URLs that will get POST'ed a payload when events related to packages happen.
All webhooks are POST to the URL with a `type` corresponding to the event being trigger and the `payload` for the given type.

## Webhook events

`PackageUpdated` 

Triggers when the package is scheduled for rebuild, has been built with a new version or failed to build.

Example:
```json
{
  "type": "PackageUpdated",
  "payload": {
    "id": 1,
    "name": "test-package",
    "run_before": "echo test",
    "status": "BUILT",
    "last_built": "2025-04-26T09:41:28Z",
    "files": [
      "test-package-1.2.3.tar.pkg.zst"
    ],
    "last_built_version": "1.2.3",
    "last_error": "When an error occurs it will show up here !"
  }
}
```

## Certificates

You can disable the verification of SSL certificates (on by default) in the config.
Alternatively you can specify a certificate to trust, also in the config.

See the server configuration docs.