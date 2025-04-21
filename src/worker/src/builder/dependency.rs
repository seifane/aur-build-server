use anyhow::{bail, Result};
use std::path::{PathBuf};
use async_recursion::async_recursion;
use futures_util::future;
use log::{debug, warn};
use petgraph::Graph;
use petgraph::graph::{EdgeIndex, NodeIndex};
use serde::Deserialize;
use srcinfo::{Srcinfo};
use crate::builder::bubblewrap::Bubblewrap;
use crate::commands::git::clone_repo;
use crate::commands::makepkg::get_src_info;
use crate::commands::pacman::is_package_in_repo;
use crate::utils::sanitize_dependency;

pub type DependencyGraph = Graph<AurPackage, ()>;

#[derive(Debug, Clone)]
pub struct AurPackage {
    pub package_name: String,
    pub package_base: String,
    pub repo_deps: Vec<String>,
}

impl PartialEq<Self> for AurPackage {
    fn eq(&self, other: &Self) -> bool {
        self.package_base == other.package_base
    }
}

pub async fn build_dependency_graph(bubblewrap: &Bubblewrap, data_path: &PathBuf, aur_package: AurPackage) -> Result<DependencyGraph> {
    let mut dep_graph = Graph::<AurPackage, ()>::new();

    let node = dep_graph.add_node(aur_package);
    get_package_dependencies(&mut dep_graph, bubblewrap, data_path, node, 0).await?;

    debug!("Dependency graph: {:#?}", dep_graph);

    Ok(dep_graph)
}

#[async_recursion]
async fn get_package_dependencies(
    dep_graph: &mut DependencyGraph,
    bubblewrap: &Bubblewrap,
    data_path: &PathBuf,
    node_index: NodeIndex,
    depth: u8,
) -> Result<()> {
    if depth > 20 {
        bail!("Max depth was reached when getting dependencies");
    }

    let node_weight = dep_graph.node_weight_mut(node_index).unwrap();

    if !data_path.join(&node_weight.package_base).exists() {
        clone_repo(&data_path, &node_weight.package_base)?;
    }
    let src_info = get_src_info(data_path, &node_weight.package_base).await?;
    debug!("Got src info: {:#?}", src_info);
    let (aur_deps, repo_deps) = extract_dependencies(bubblewrap, &src_info).await;
    debug!("Got aur deps: {:#?}", aur_deps);
    debug!("Got repo deps: {:#?}", repo_deps);
    node_weight.repo_deps = repo_deps;

    for aur_dep in aur_deps {
        if let Some(found) = dep_graph.node_indices().find(|n| dep_graph[*n] == aur_dep) {
            warn!("Found a dependency that was already included in the graph: {:?} dep of {:?}", aur_dep, &dep_graph[node_index]);
            add_edge(dep_graph, node_index, found);
        } else {
            let dep_node = dep_graph.add_node(aur_dep);
            if add_edge(dep_graph, node_index, dep_node).is_some() {
                get_package_dependencies(dep_graph, bubblewrap, data_path, dep_node, depth + 1).await?
            }
        };
    }

    Ok(())
}

fn add_edge(dep_graph: &mut DependencyGraph, start: NodeIndex, end: NodeIndex) -> Option<EdgeIndex> {
    if dep_graph.find_edge(start, end).is_some() {
        warn!("Dependency is path was already added, not adding again");
        None
    } else if dep_graph.find_edge(end, start).is_some() {
        warn!("Circular dependency detected not adding edge");
        None
    } else {
        Some(dep_graph.add_edge(start, end, ()))
    }
}

async fn get_aur_dependencies(bubblewrap: &Bubblewrap, deps: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut aur_dependency = Vec::new();
    let mut repo_dependency = Vec::new();

    for dep in deps.into_iter() {
        if !is_package_in_repo(bubblewrap, &dep).await {
            aur_dependency.push(dep);
        } else {
            repo_dependency.push(dep);
        }
    }

    (aur_dependency, repo_dependency)
}

async fn extract_dependencies(
    bubblewrap: &Bubblewrap,
    srcinfo: &Srcinfo,
) -> (Vec<AurPackage>, Vec<String>)
{
    let mut dependencies = Vec::new();

    let mut packages = vec![srcinfo.base.pkgbase.clone()];

    dependencies.append(&mut srcinfo.base.makedepends.clone()
        .into_iter()
        .flat_map(|v| v.vec)
        .collect());
    dependencies.append(&mut srcinfo.base.checkdepends.clone()
        .into_iter()
        .flat_map(|v| v.vec)
        .collect());

    for pkg in srcinfo.pkgs.iter() {
        packages.push(pkg.pkgname.clone());

        dependencies.append(&mut pkg.depends.clone().into_iter().flat_map(|v| v.vec).collect());
    }

    dependencies = dependencies.into_iter().map(|d| sanitize_dependency(d.as_str())).collect();
    dependencies.dedup();
    dependencies.retain(|d| !packages.contains(d));

    let (aur_dependencies, repo_dependencies) = get_aur_dependencies(bubblewrap, dependencies).await;

    let mut aur_dependency = future::join_all(aur_dependencies.iter().map(|i| aur_api_query_provides(i))).await;
    aur_dependency.retain(|i| !packages.contains(&i.package_base));

    debug!("Found AUR dependencies for {} {:?}", srcinfo.base.pkgbase, aur_dependency);

    (aur_dependency, repo_dependencies)
}

#[derive(Deserialize)]
struct AurResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PackageBase")]
    pub package_base: String,
}

#[derive(Deserialize)]
struct AurResults {
    pub results: Vec<AurResult>
}

pub async fn aur_api_query_provides(package_name: &String) -> AurPackage
{
    let body = reqwest::get(format!(
        "https://aur.archlinux.org/rpc/v5/search/{}?by=provides", package_name
    )).await;

    if let Ok(response) = body {
        let parsed: Result<AurResults, reqwest::Error> = response.json().await;
        if let Ok(parsed) = parsed {
            for result in parsed.results.iter() {
                if &result.name == package_name {
                    return AurPackage {
                        package_name: package_name.clone(),
                        package_base: result.package_base.clone(),
                        repo_deps: Vec::new(),
                    }
                }
            }

            if !parsed.results.is_empty() {
                warn!("Did not find exact name match for {}, falling back to picking first option", package_name);
                return AurPackage {
                    package_name: package_name.clone(),
                    package_base: parsed.results[0].package_base.clone(),
                    repo_deps: Vec::new(),
                };
            }

        }
    }

    warn!("Could not find {} by provides, falling back to fetching by name", package_name);
    AurPackage {
        package_name: package_name.clone(),
        package_base: package_name.clone(),
        repo_deps: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use log::LevelFilter;
    use petgraph::Direction;
    use petgraph::prelude::EdgeRef;
    use serial_test::serial;
    use crate::builder::bubblewrap::Bubblewrap;
    use crate::builder::dependency::{AurPackage, build_dependency_graph};
    use crate::models::config::Config;

    fn get_config() -> Config {
        Config {
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
        }
    }

    fn get_bubblewrap() -> Bubblewrap {
        Bubblewrap::new(
            PathBuf::from("./test/sandbox"),
            PathBuf::from("../../config/pacman.conf"),
            PathBuf::from("../../config/mirrorlist")
        )
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    pub async fn can_build_graph() {
        let bubblewrap = get_bubblewrap();
        bubblewrap.create(true).await.unwrap();

        let graph = build_dependency_graph(
            &bubblewrap,
            &PathBuf::from("./test/sandbox"),
            AurPackage {
                package_name: "bottles".to_string(),
                package_base: "bottles".to_string(),
                repo_deps: Vec::new(),
            },
        ).await.unwrap();
        println!("{:#?}", graph);

        let root = graph.node_indices().find(|ni| graph[*ni].package_name.as_str() == "bottles").unwrap();
        let edges = graph.edges_directed(root, Direction::Outgoing);

        let mut first_level_deps = Vec::new();
        for edge in edges {
            let node = &graph[edge.target()];
            first_level_deps.push(node.package_name.clone());
        }

        assert_eq!(first_level_deps, vec![
            format!("vkbasalt-cli"),
            format!("python-steamgriddb"),
            format!("python-pathvalidate"),
            format!("python-fvs"),
            format!("patool"),
            format!("icoextract"),
        ]);

        println!("first_level_deps {:?}", first_level_deps);
        println!("Generated graph for bottles {:?}", graph);
    }
}