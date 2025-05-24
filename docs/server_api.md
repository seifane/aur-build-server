# API

The API is accessible through the port configured for the server.

## API Authentication
The API is protected using an API key specified in the `config_server.json` file.
You can authenticate a request by including the API key in the `Authorization` header.

## Endpoints

| Method | Path                | Description                        | Payload                                         | Response                                      |
|--------|---------------------|------------------------------------|-------------------------------------------------|-----------------------------------------------|
| GET    | /workers            | List workers                       | N/A                                             | [WorkerResponse[]](#WorkerResponse)           |
| DELETE | /workers/{id}       | Delete a worker                    | N/A                                             | [SuccessResponse](#SuccessResponse)           |
| GET    | /packages           | List packages                      | N/A                                             | [PackageResponse[]](#PackageResponse)         |
| POST   | /packages           | Create a new package               | [CreatePackagePayload](#CreatePackagePayload)   | [PackageResponse](#PackageResponse)           |
| POST   | /packages/rebuild   | Rebuild packages                   | [PackageRebuildPayload](#PackageRebuildPayload) | [SuccessResponse](#SuccessResponse)           |
| PATCH  | /packages/{id}      | Update a package                   | [UpdatePackagePayload](#UpdatePackagePayload)   | [PackageResponse](#PackageResponse)           |
| DELETE | /packages/{id}      | Delete a package                   | N/A                                             | [SuccessResponse](#SuccessResponse)           |
| GET    | /packages/{id}/logs | Get build logs for a package       | N/A                                             | Text file containing the logs for the package |
| POST   | /webhooks/trigger   | Trigger a fake webhook for testing | N/A                                             | [SuccessResponse](#SuccessResponse)           |

### Responses

#### SuccessResponse
```rust
pub struct SuccessResponse {
    pub success: bool,
}
```

#### WorkerResponse
```rust
pub struct WorkerResponse {
    pub id: usize,
    pub status: WorkerStatus,
    pub current_job: Option<String>,
    pub version: String,
}
```

#### PackageResponse
```rust
pub struct PackageResponse {
    pub id: i32,
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub files: Vec<String>,
    pub last_built_version: Option<String>,
    pub last_error: Option<String>,
}
```

### Payloads

#### CreatePackagePayload
```rust
pub struct CreatePackagePayload {
    pub name: String,
    pub run_before: Option<String>,
}
```

#### PackageRebuildPayload
```rust
pub struct PackageRebuildPayload {
    pub packages: Option<Vec<i32>>,
    pub force: Option<bool>
}
```

#### UpdatePackagePayload
```rust
pub struct UpdatePackagePayload {
  pub run_before: Option<String>,
}
```