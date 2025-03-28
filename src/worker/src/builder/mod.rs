use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use log::{error, info};
use petgraph::Direction;
use tokio::sync::mpsc::Sender;

use common::models::{PackageJob, WorkerStatus};

use crate::builder::bubblewrap::Bubblewrap;
use crate::builder::dependency::{aur_api_query_provides, AurPackage, build_dependency_graph, DependencyGraph};
use crate::builder::utils::post_build_clean;
use crate::commands::git::{apply_patches, clone_repo};
use crate::commands::gpg::attempt_recv_gpg_keys;
use crate::commands::makepkg::{get_package_version, run_makepkg};
use crate::commands::pacman::{pacman_update_repos};
use crate::logs::{init_builder_logs, LogSection, write_log_section};
use crate::models::config::Config;
use crate::models::package_build_result::PackageBuildResult;
use crate::orchestrator::http::HttpClient;
use crate::utils::{copy_dir_all, get_package_dir_entries};

pub mod bubblewrap;
mod dependency;
mod utils;

pub struct Builder {
    bubblewrap: Bubblewrap,
    package_job: PackageJob,

    tx_status: Sender<WorkerStatus>,
    http_client: HttpClient,

    config: Config
}

impl Builder {
    pub fn new(tx_status: Sender<WorkerStatus>, http_client: HttpClient, package_job: PackageJob, config: &Config) -> Builder {
        Builder {
            bubblewrap: Bubblewrap::from_config(&config),
            package_job,

            tx_status,
            http_client,

            config: config.clone()
        }
    }

    async fn fetch_package(&self) -> Result<AurPackage>
    {
        let parent_package = aur_api_query_provides(&self.package_job.definition.name).await;
        let repository = clone_repo(&self.config.data_path, &parent_package.package_base)?;
        apply_patches(&self.package_job, repository).await?;
        Ok(parent_package)
    }

    async fn handle_dependencies(&self, dep_graph: &mut DependencyGraph) -> Result<()> {
        while dep_graph.raw_nodes().len() > 1 {
            let start_size = dep_graph.raw_nodes().len();
            for i in dep_graph.node_indices() {
                let mut edges = dep_graph.edges_directed(i, Direction::Outgoing);
                if edges.next().is_none() {
                    let node = &dep_graph[i];

                    info!("Building AUR dependency {}", node.package_base);
                    self.build_package(node).await?;
                    info!("Built AUR dependency {}", node.package_base);

                    dep_graph.remove_node(i);
                    break;
                }
            }
            if start_size == dep_graph.raw_nodes().len() {
                bail!("Dependency graph did not reduce after iteration, this most likely means a circular dependency is present");
            }
        }
        Ok(())
    }

    async fn handle_package(&self, dep_graph: &mut DependencyGraph) -> Result<()> {
        if dep_graph.raw_nodes().len() != 1 {
            bail!("Received unexpected number of node in dependency graph {}", dep_graph.raw_nodes().len());
        }

        let index = dep_graph.node_indices().next().unwrap();
        let node = &dep_graph[index];
        if &node.package_name != &self.package_job.definition.name {
            bail!("Fatal: last element in graph not the main package {} != {}", node.package_name, self.package_job.definition.name);
        }

        self.build_package(&node).await?;

        Ok(())
    }

    async fn build_package(
        &self,
        package: &AurPackage,
    ) -> Result<()> {
        info!("Building package {}", package.package_base);

        let root = self.init_build_chroot().await.with_context(|| "Failed to init build chroot")?;

        info!("Copying package into chroot");
        copy_dir_all(self.config.data_path.join(&package.package_base), root.join("package")).await
            .with_context(|| "Failed to copy package into chroot")?;

        info!("Attempting to fetch GPG keys");
        attempt_recv_gpg_keys(&self.bubblewrap, &self.config.data_path, &package.package_base).await;

        if let Some(run_before) = self.package_job.definition.run_before.as_ref() {
            info!("Running run_before command '{}'", run_before);
            let output = self.bubblewrap.run_sandbox_fakeroot("current", "/", "sh", vec!["-c", run_before.as_str()]).await
                .with_context(|| format!("Failed to execute run_before for {}", package.package_base))?;
            write_log_section(&self.config.build_logs_path, &self.package_job.definition.name, LogSection::RunBeforeOut, output.stdout.as_slice()).await?;
            write_log_section(&self.config.build_logs_path, &self.package_job.definition.name, LogSection::RunBeforeErr, output.stderr.as_slice()).await?;
        }

        let output = run_makepkg(&self.bubblewrap, &package.package_base).await
            .with_context(|| format!("Error while running makepkg for {}", &package.package_base))?;
        write_log_section(&self.config.build_logs_path, &self.package_job.definition.name, LogSection::MakePkgOut(package.package_base.clone()), output.stdout.as_slice()).await?;
        write_log_section(&self.config.build_logs_path, &self.package_job.definition.name, LogSection::MakePkgErr(package.package_base.clone()), output.stderr.as_slice()).await?;

        if !output.status.success() {
            bail!("Failed to run makepkg for {}", package.package_base);
        }

        self.bubblewrap.copy_built_packages(self.config.data_path.join("_built")).await
            .with_context(|| format!("Failed to copy packages for {}", package.package_base))?;

        Ok(())
    }

    async fn stage_init(&self) -> Result<AurPackage> {
        post_build_clean(&self.config.data_path).await.with_context(|| "Failed initial clean")?;

        init_builder_logs(&self.config.build_logs_path).await.with_context(|| "Failed to init logs")?;

        info!("Fetch package");
        let package = self.fetch_package().await.with_context(|| "Failed to fetch package")?;

        Ok(package)
    }

    async fn init_build_chroot(&self) -> Result<PathBuf>
    {
        let deps = get_package_dir_entries(self.config.data_path.join("_built")).await?
            .iter()
            .map(|e| e.path())
            .collect::<Vec<_>>();

        let root = if !deps.is_empty() {
            self.bubblewrap.create_from_base_install_packages("current", deps).await?
        } else {
            self.bubblewrap.create_from_base("current").await?
        };

        Ok(root)
    }

    async fn stage_build(&self, aur_package: AurPackage) -> Result<()>
    {
        info!("Building dependency graph");
        let mut dep_graph = build_dependency_graph(&self.bubblewrap, &self.config.data_path, aur_package).await
            .with_context(|| "Failed to build dependency graph")?;

        self.handle_dependencies(&mut dep_graph).await?;
        self.handle_package(&mut dep_graph).await?;

        Ok(())
    }

    pub async fn try_process_package(&self) -> Result<PackageBuildResult> {
        self.tx_status.send(WorkerStatus::INIT).await.unwrap();
        let aur_package = self.stage_init().await?;

        info!("Checking package version");
        let version = get_package_version(&self.config.data_path, &aur_package.package_base).await?;
        if let Some(last_built_version) = &self.package_job.last_built_version {
            if last_built_version == &version {
                info!("Found same version for package, skipping build ...");
                return Ok(PackageBuildResult::new(false, version));
            }
        }

        self.tx_status.send(WorkerStatus::UPDATING).await.unwrap();
        info!("Updating base chroot");
        pacman_update_repos(&self.bubblewrap).await?;

        self.tx_status.send(WorkerStatus::WORKING).await.unwrap();
        self.stage_build(aur_package).await?;

        Ok(PackageBuildResult::new(true, version))
    }

    pub async fn process_package(&self) -> Result<()>
    {
        info!("Starting to process package {}", self.package_job.definition.name);

        let build_result = self.try_process_package().await;
        info!("Package build result {:?}", build_result);

        if let Err(e) = build_result.as_ref() {
            error!("Package build error: {}", e);
        }

        self.tx_status.send(WorkerStatus::UPLOADING).await?;
        self.send_job_result(build_result).await?;

        self.tx_status.send(WorkerStatus::CLEANING).await?;
        post_build_clean(&self.config.data_path).await?;
        Ok(())
    }

    async fn send_job_result(
        &self,
        build_result: Result<PackageBuildResult>,
    ) -> Result<()>
    {
        info!("Sending job result to orchestrator");
        self.http_client.upload_packages(&self.package_job.definition.name, build_result).await?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use log::LevelFilter;
    use serial_test::serial;
    use simplelog::{ColorChoice, TerminalMode, TermLogger};
    use tokio::sync::mpsc::channel;
    use anyhow::Result;

    use common::models::{PackageDefinition, PackageJob};
    use crate::builder::bubblewrap::Bubblewrap;

    use crate::builder::Builder;
    use crate::builder::utils::post_build_clean;
    use crate::models::config::Config;
    use crate::models::package_build_result::PackageBuildResult;
    use crate::orchestrator::http::HttpClient;

    async fn build_package(package_job: PackageJob) -> Result<PackageBuildResult> {
        let config: Config = Config {
            log_level: LevelFilter::Debug,
            log_path: PathBuf::from("./test/worker.log"),
            pacman_config_path: PathBuf::from("../../config/pacman.conf"),
            pacman_mirrorlist_path: PathBuf::from("../../config/mirrorlist"),
            force_base_sandbox_create: false,
            data_path: PathBuf::from("./test/data"),
            sandbox_path: PathBuf::from("./test/sandbox"),
            build_logs_path: PathBuf::from("./test/build_logs"),
            base_url: "".to_string(),
            base_url_ws: "".to_string(),
            api_key: "".to_string(),
        };

        TermLogger::init(LevelFilter::Debug, simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

        post_build_clean(&config.data_path).await.unwrap();
        let bubblewrap = Bubblewrap::from_config(&config);
        bubblewrap.create(false).await.unwrap();

        let (tx, _rx) = channel(1000);
        let builder = Builder::new(
            tx,
            HttpClient::from_config(&config),
            package_job,
            &config
        );

        builder.try_process_package().await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    pub async fn test_dependencies_in_pkgbase() {
        let job = PackageJob {
            definition: PackageDefinition {
                package_id: 1,
                name: "phpstorm".to_string(),
                run_before: None,
                patches: None,
            },
            last_built_version: None,
        };

        let result = build_package(job).await.unwrap();
        assert!(result.built);
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    pub async fn test_build_package() {
        let job = PackageJob {
            definition: PackageDefinition {
                package_id: 1,
                name: "flutter".to_string(),
                run_before: None,
                patches: None,
            },
            last_built_version: None,
        };

        let result = build_package(job).await.unwrap();
        assert!(result.built);
    }
}