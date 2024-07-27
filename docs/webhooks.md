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
    "package": {
      "name": "google-chrome",
      "run_before": null,
      "patches": null
    },
    "status": "BUILT",
    "last_built": "2024-01-01T00:00:00",
    "last_built_version": "1.2.3",
    "last_error": null
  }
}
```