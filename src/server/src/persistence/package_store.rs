use crate::persistence::schema;
use anyhow::Result;
use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{AsChangeset, AsExpression, Connection, ExpressionMethods, FromSqlRow, Insertable, OptionalExtension, QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper, SqliteConnection, TextExpressionMethods};
use std::ops::{DerefMut, Sub};
use std::path::PathBuf;
use std::sync::{Arc};
use chrono::{DateTime, TimeDelta, Utc};
use diesel::sqlite::Sqlite;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::info;
use tokio::sync::Mutex;
use common::http::responses::{PackagePatchResponse, PackageResponse};
use common::models::{PackageDefinition, PackageJob, PackagePatchDefinition, PackageStatus};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Debug, AsExpression, FromSqlRow, Clone)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct StringArray(Vec<String>);

impl ToSql<Text, Sqlite> for StringArray {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> diesel::serialize::Result {
        out.set_value(serde_json::to_string(&self.0)?);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for StringArray {
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let vec: Vec<String> = serde_json::from_reader(bytes.read_blob())?;
        Ok(StringArray(vec))
    }
}

#[derive(Queryable, Selectable, Debug, AsChangeset)]
#[diesel(table_name = schema::packages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Package {
    id: i32,
    name: String,
    pub run_before: Option<String>,
    status: i16,
    last_built: Option<i64>,
    files: StringArray,
    pub last_built_version: Option<String>,
    pub last_error: Option<String>
}

impl Package {
    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_status(&self) -> PackageStatus {
        PackageStatus::from_u8(self.status as u8)
    }

    pub fn set_status(&mut self, status: PackageStatus) {
        self.status = status as u8 as i16;
    }

    pub fn get_last_built(&self) -> Option<DateTime<Utc>> {
        self.last_built.map(|ts| DateTime::from_timestamp(ts, 0).unwrap())
    }

    pub fn set_last_built(&mut self, last_built: Option<DateTime<Utc>>) {
        self.last_built = last_built.map(|t| t.timestamp());
    }

    pub fn get_files(&self) -> &Vec<String>
    {
        &self.files.0
    }

    pub fn get_files_mut(&mut self) -> &mut Vec<String> {
        &mut self.files.0
    }

    pub fn get_package_job(&self, patches: Vec<PackagePatch>) -> PackageJob {
        PackageJob {
            definition: PackageDefinition {
                package_id: self.id,
                name: self.name.clone(),
                run_before: self.run_before.clone(),
                patches: patches.into_iter().map(Into::into).collect(),
            },
            last_built_version: self.last_built_version.clone(),
        }
    }
}

impl Into<PackageResponse> for Package {
    fn into(self) -> PackageResponse {
        PackageResponse {
            id: self.get_id(),
            name: self.get_name().to_string(),
            status: self.get_status(),
            last_built: self.get_last_built(),
            files: self.get_files().to_vec(),
            run_before: self.run_before,
            last_built_version: self.last_built_version,
            last_error: self.last_error,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::packages)]
pub struct PackageInsert {
    pub name: String,
    pub run_before: Option<String>,
}

#[derive(Queryable, Selectable, Debug, AsChangeset)]
#[diesel(table_name = schema::package_patches)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PackagePatch {
    id: i32,
    pub package_id: i32,
    pub url: String,
    pub sha_512: Option<String>
}

impl PackagePatch {
    pub fn get_id(&self) -> i32
    {
        self.id
    }
}

impl Into<PackagePatchDefinition> for PackagePatch {
    fn into(self) -> PackagePatchDefinition {
        PackagePatchDefinition {
            url: self.url,
            sha512: self.sha_512,
        }
    }
}

impl Into<PackagePatchResponse> for PackagePatch {
    fn into(self) -> PackagePatchResponse {
        PackagePatchResponse {
            id: self.id,
            package_id: self.package_id,
            url: self.url,
            sha_512: self.sha_512,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::package_patches)]
pub struct PackagePatchInsert {
    pub package_id: i32,
    pub url: String,
    pub sha_512: Option<String>
}

pub struct PackageStore {
    connection: Arc<Mutex<SqliteConnection>>
}

impl PackageStore {
    pub fn from_disk(path: PathBuf) -> Result<Self> {
        let connection = Arc::new(Mutex::new(SqliteConnection::establish(path.to_str().unwrap())?));
        Ok(PackageStore { connection })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let connection = Arc::new(Mutex::new(SqliteConnection::establish(":memory:")?));
        Ok(PackageStore { connection })
    }

    pub async fn run_migrations(&mut self) -> Result<()> {
        info!("Running migrations");
        self.connection.lock().await.run_pending_migrations(MIGRATIONS).unwrap();
        Ok(())
    }

    pub async fn create_package(&mut self, insert: PackageInsert) -> Result<Package> {
        let package = diesel::insert_into(schema::packages::table)
            .values(&insert)
            .returning(Package::as_returning())
            .get_result(self.connection.lock().await.deref_mut())?;
        Ok(package)
    }

    pub async fn update_package(&mut self, package: &Package) -> Result<Package>
    {
        let package = diesel::update(schema::packages::table)
            .filter(schema::packages::id.eq(package.id))
            .set(package)
            .returning(Package::as_returning())
            .get_result(self.connection.lock().await.deref_mut())?;
        Ok(package)
    }

    pub async fn update_package_status(&mut self, id: i32, status: PackageStatus) -> Result<()>
    {
        diesel::update(schema::packages::table)
            .filter(schema::packages::id.eq(id))
            .set(schema::packages::status.eq(status as u8 as i16))
            .execute(self.connection.lock().await.deref_mut())?;
        Ok(())
    }

    pub async fn set_packages_pending(&mut self, package_ids: Option<Vec<i32>>, force: bool) -> Result<()> {
        let package_ids = match package_ids {
            Some(ids) => ids,
            None => self.get_packages().await?.into_iter().map(|p| p.id).collect()
        };

        if force {
            diesel::update(schema::packages::table)
                .filter(schema::packages::id.eq_any(package_ids))
                .set((
                    schema::packages::status.eq(PackageStatus::PENDING as u8 as i16),
                    schema::packages::last_built_version.eq(None::<String>),
                ))
                .execute(self.connection.lock().await.deref_mut())?;
        } else {
            diesel::update(schema::packages::table)
                .filter(schema::packages::id.eq_any(package_ids))
                .filter(schema::packages::status.ne::<i16>(PackageStatus::BUILDING.into()))
                .set((
                    schema::packages::status.eq::<i16>(PackageStatus::PENDING.into()),
                ))
                .execute(self.connection.lock().await.deref_mut())?;
        };
        Ok(())
    }

    pub async fn set_packages_rebuild(&mut self, rebuild_interval: i64) -> Result<usize> {
        let cutoff = Utc::now().sub(TimeDelta::seconds(rebuild_interval)).timestamp();

        let res = diesel::update(schema::packages::table)
            .filter(schema::packages::last_built.lt(cutoff))
            .filter(schema::packages::status.eq_any::<Vec<i16>>(vec![
                PackageStatus::FAILED.into(),
                PackageStatus::BUILT.into(),
            ]))
            .set(schema::packages::status.eq::<i16>(PackageStatus::PENDING.into()))
            .execute(self.connection.lock().await.deref_mut())?;
        Ok(res)
    }

    pub async fn delete_package(&mut self, id: i32) -> Result<()>
    {
        diesel::delete(schema::packages::table)
            .filter(schema::packages::id.eq(id))
            .execute(self.connection.lock().await.deref_mut())?;
        Ok(())
    }

    pub async fn get_packages(&mut self) -> Result<Vec<Package>> {
        let packages = schema::packages::dsl::packages
            .order(schema::packages::id.asc())
            .select(Package::as_select())
            .load::<Package>(self.connection.lock().await.deref_mut())?;
        Ok(packages)
    }

    pub async fn search_packages_by_name(&mut self, search: String) -> Result<Vec<Package>>
    {
        let packages = schema::packages::dsl::packages
            .order(schema::packages::id.asc())
            .filter(schema::packages::name.like(format!("%{}%", search)))
            .select(Package::as_select())
            .load::<Package>(self.connection.lock().await.deref_mut())?;

        Ok(packages)
    }

    pub async fn get_package(&mut self, id: i32) -> Result<Option<Package>>
    {
        let package = schema::packages::dsl::packages
            .filter(schema::packages::id.eq(id))
            .first::<Package>(self.connection.lock().await.deref_mut())
            .optional()?;
        Ok(package)
    }

    pub async fn get_package_by_name(&mut self, name: &str) -> Result<Option<Package>>
    {
        let package = schema::packages::dsl::packages
            .filter(schema::packages::name.eq(name))
            .first::<Package>(self.connection.lock().await.deref_mut())
            .optional()?;
        Ok(package)
    }

    pub async fn get_next_pending_package(&mut self) -> Result<Option<Package>>
    {
        let package = schema::packages::dsl::packages
            .order(schema::packages::id.asc())
            .filter(schema::packages::status.eq(PackageStatus::PENDING as i16))
            .limit(1)
            .select(Package::as_select())
            .first::<Package>(self.connection.lock().await.deref_mut()).optional()?;
        Ok(package)
    }

    pub async fn create_patch(&mut self, patch: PackagePatchInsert) -> Result<PackagePatch> {
        let patch = diesel::insert_into(schema::package_patches::table)
            .values(&patch)
            .returning(PackagePatch::as_returning())
            .get_result(self.connection.lock().await.deref_mut())?;
        Ok(patch)
    }

    pub async fn update_patch(&mut self, patch: &PackagePatch) -> Result<PackagePatch> {
        let patch = diesel::update(schema::package_patches::table)
            .filter(schema::package_patches::id.eq(patch.id))
            .set(patch)
            .returning(PackagePatch::as_returning())
            .get_result(self.connection.lock().await.deref_mut())?;
        Ok(patch)
    }

    pub async fn delete_patch(&mut self, id: i32) -> Result<()> {
        diesel::delete(schema::package_patches::table)
            .filter(schema::package_patches::id.eq(id))
            .execute(self.connection.lock().await.deref_mut())?;
        Ok(())
    }

    pub async fn get_patches_for_package(&mut self, package_id: i32) -> Result<Vec<PackagePatch>>
    {
        let patches = schema::package_patches::dsl::package_patches
            .filter(schema::package_patches::package_id.eq(package_id))
            .select(PackagePatch::as_select())
            .load::<PackagePatch>(self.connection.lock().await.deref_mut())?;

        Ok(patches)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};
    use common::models::PackageStatus;
    use crate::persistence::package_store::{PackageInsert, PackagePatchInsert, PackageStore};

    async fn get_instance() -> PackageStore {
        let mut package_repository = PackageStore::in_memory().unwrap();
        package_repository.run_migrations().await.unwrap();

        package_repository
    }

    #[tokio::test]
    async fn test_create_package() {
        let mut package_repository = get_instance().await;

        let package = package_repository.create_package(PackageInsert {
            name: "Name".to_string(),
            run_before: Some("echo 1".to_string()),
        }).await.unwrap();
        assert_eq!(1, package.id);
        assert_eq!("Name", package.name);
        assert_eq!("echo 1", package.run_before.as_ref().unwrap());
        assert_eq!(PackageStatus::PENDING, package.get_status());
    }

    #[tokio::test]
    async fn test_update_package() {
        let mut package_repository = get_instance().await;

        let mut package = package_repository.create_package(PackageInsert {
            name: "Name".to_string(),
            run_before: Some("echo 1".to_string()),
        }).await.unwrap();
        let built_time = Utc::now();

        package.set_status(PackageStatus::BUILT);
        package.set_last_built(Some(built_time));
        package.files.0.push("first_file".to_string());
        package.files.0.push("second_file".to_string());
        package.last_built_version = Some("last_version".to_string());
        package.last_error = Some("last_error".to_string());
        package_repository.update_package(&package).await.unwrap();

        let packages = package_repository.get_packages().await.unwrap();

        assert_eq!(1, packages.len());
        assert_eq!(1, packages[0].id);
        assert_eq!("Name", packages[0].name);
        assert_eq!("echo 1", packages[0].run_before.as_ref().unwrap());
        assert_eq!(PackageStatus::BUILT, packages[0].get_status());
        assert_eq!(2, packages[0].files.0.len());
        assert_eq!("first_file", packages[0].files.0[0]);
        assert_eq!("second_file", packages[0].files.0[1]);
        assert_eq!(Some("last_version".to_string()), packages[0].last_built_version);
        assert_eq!(Some("last_error".to_string()), packages[0].last_error);
    }

    #[tokio::test]
    async fn test_update_package_status() {
        let mut package_repository = get_instance().await;

        let package = package_repository.create_package(PackageInsert {
            name: "name".to_string(),
            run_before: None,
        }).await.unwrap();

        assert_ne!(PackageStatus::BUILT, package.get_status());

        package_repository.update_package_status(1, PackageStatus::BUILT).await.unwrap();
        let package = package_repository.get_package_by_name("name").await.unwrap().unwrap();
        assert_eq!(PackageStatus::BUILT, package.get_status());
    }

    #[tokio::test]
    async fn test_set_packages_pending() {
        let mut package_repository = get_instance().await;

        package_repository.create_package(PackageInsert {
            name: "first".to_string(),
            run_before: None,
        }).await.unwrap();
        package_repository.create_package(PackageInsert {
            name: "second".to_string(),
            run_before: None,
        }).await.unwrap();

        package_repository.update_package_status(1, PackageStatus::BUILT).await.unwrap();
        package_repository.update_package_status(2, PackageStatus::BUILDING).await.unwrap();

        package_repository.set_packages_pending(Some(vec![1, 2]), false).await.unwrap();

        assert_eq!(
            PackageStatus::PENDING,
            package_repository.get_package_by_name("first").await.unwrap().unwrap().get_status()
        );
        let mut second = package_repository
            .get_package_by_name("second").await.unwrap().unwrap();
        assert_eq!(
            PackageStatus::BUILDING,
            second.get_status()
        );
        second.last_built_version = Some("last_version".to_string());
        package_repository.update_package(&second).await.unwrap();

        package_repository.set_packages_pending(Some(vec![1, 2]), true).await.unwrap();

        let second = package_repository
            .get_package_by_name("second").await.unwrap().unwrap();
        assert_eq!(PackageStatus::PENDING, second.get_status());
        assert!(second.last_built_version.is_none());
    }

    #[tokio::test]
    async fn test_set_packages_rebuild() {
        let mut package_repository = get_instance().await;

        let mut package = package_repository.create_package(PackageInsert {
            name: "first".to_string(),
            run_before: None,
        }).await.unwrap();
        package.set_status(PackageStatus::BUILT);
        package.set_last_built(Some(Utc::now()));
        package_repository.update_package(&package).await.unwrap();

        package_repository.set_packages_rebuild(100).await.unwrap();

        let mut package = package_repository.get_package_by_name("first").await.unwrap().unwrap();
        assert_eq!(PackageStatus::BUILT, package.get_status());
        package.set_last_built(Some(Utc::now() - TimeDelta::seconds(200)));
        package_repository.update_package(&package).await.unwrap();

        package_repository.set_packages_rebuild(100).await.unwrap();

        let mut package = package_repository.get_package_by_name("first").await.unwrap().unwrap();
        assert_eq!(PackageStatus::PENDING, package.get_status());
        package.set_status(PackageStatus::BUILDING);
        package.set_last_built(Some(Utc::now() - TimeDelta::seconds(200)));
        package_repository.update_package(&package).await.unwrap();

        package_repository.set_packages_rebuild(100).await.unwrap();

        let package = package_repository.get_package_by_name("first").await.unwrap().unwrap();
        assert_eq!(PackageStatus::BUILDING, package.get_status());
    }

    #[tokio::test]
    async fn test_delete_package() {
        let mut package_repository = get_instance().await;

        package_repository.create_package(PackageInsert {
            name: "first".to_string(),
            run_before: Some("echo 1".to_string()),
        }).await.unwrap();
        package_repository.create_package(PackageInsert {
            name: "second".to_string(),
            run_before: None,
        }).await.unwrap();

        package_repository.delete_package(1).await.unwrap();

        assert_eq!(1, package_repository.get_packages().await.unwrap().len());
        assert!(package_repository.get_package_by_name("first").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_packages() {
        let mut package_repository = get_instance().await;

        package_repository.create_package(PackageInsert {
            name: "first".to_string(),
            run_before: Some("echo 1".to_string()),
        }).await.unwrap();
        package_repository.create_package(PackageInsert {
            name: "second".to_string(),
            run_before: None,
        }).await.unwrap();

        let packages = package_repository.get_packages().await.unwrap();
        assert_eq!(2, packages.len());
        assert_eq!("first", packages[0].name);
        assert_eq!(Some("echo 1".to_string()), packages[0].run_before);
        assert_eq!("second", packages[1].name);
        assert_eq!(None, packages[1].run_before);

        let package = package_repository.get_package_by_name("first").await.unwrap();
        assert_eq!(1, package.unwrap().id);

        let package = package_repository.get_package_by_name("none").await.unwrap();
        assert!(package.is_none());

        let package = package_repository.get_next_pending_package().await.unwrap();
        assert_eq!("first", package.unwrap().name);
    }

    #[tokio::test]
    async fn test_create_list_delete_patch() {
        let mut package_repository = get_instance().await;

        let package = package_repository.create_package(PackageInsert {
            name: "package".to_string(),
            run_before: None,
        }).await.unwrap();
        package_repository.create_patch(PackagePatchInsert {
            package_id: package.id,
            url: "test_url".to_string(),
            sha_512: None,
        }).await.unwrap();

        let patches = package_repository.get_patches_for_package(package.id).await.unwrap();
        assert_eq!(1, patches.len());
        assert_eq!("test_url", patches[0].url);

        package_repository.delete_patch(patches[0].id).await.unwrap();
        assert_eq!(0, package_repository.get_patches_for_package(package.id).await.unwrap().len());
        assert_eq!(1, package_repository.get_packages().await.unwrap().len());
    }

    #[tokio::test]
    async fn test_update_patch() {
        let mut package_repository = get_instance().await;

        let package = package_repository.create_package(PackageInsert {
            name: "package".to_string(),
            run_before: None,
        }).await.unwrap();
        let mut patch = package_repository.create_patch(PackagePatchInsert {
            package_id: package.id,
            url: "test_url".to_string(),
            sha_512: Some("sha".to_string()),
        }).await.unwrap();
        patch.url = "test_url_update".to_string();
        package_repository.update_patch(&patch).await.unwrap();

        let patches = package_repository.get_patches_for_package(package.id).await.unwrap();
        assert_eq!(1, patches.len());
        assert_eq!("test_url_update", patches[0].url);
        assert_eq!(Some("sha".to_string()), patches[0].sha_512);
    }
}