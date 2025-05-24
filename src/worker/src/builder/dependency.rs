use anyhow::{anyhow, bail, Error, Result};
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
    let (aur_deps, repo_deps) = extract_dependencies(bubblewrap, &src_info).await?;
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

async fn split_aur_dependencies(bubblewrap: &Bubblewrap, deps: Vec<String>) -> (Vec<String>, Vec<String>) {
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
) -> Result<(Vec<AurPackage>, Vec<String>)>
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

    dependencies.dedup();
    dependencies.retain(|d| !packages.contains(d));

    let (aur_dependencies, repo_dependencies) = split_aur_dependencies(bubblewrap, dependencies).await;

    let mut aur_packages = future::join_all(
        aur_dependencies.iter().map(|i| async move {
            aur_api_query_provides(i, false).await.ok_or(anyhow!("Failed to get aur dependency {} by provide", i))
        })
    ).await.into_iter().collect::<Result<Vec<AurPackage>, Error>>()?;
    aur_packages.retain(|i| !packages.contains(&i.package_base));

    debug!("Found AUR dependencies for {} {:?}", srcinfo.base.pkgbase, aur_packages);

    Ok((aur_packages, repo_dependencies))
}

#[derive(Deserialize, Debug)]
struct AurResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PackageBase")]
    pub package_base: String,
}

#[derive(Deserialize, Debug)]
struct AurResults {
    pub results: Vec<AurResult>
}

pub async fn aur_api_query_provides(package_name: &String, strict: bool) -> Option<AurPackage>
{
    let sanitized_package_name = sanitize_dependency(package_name);
    let url = format!(
        "https://aur.archlinux.org/rpc/v5/search/{}?by=provides",
        sanitized_package_name
    );

    let body = reqwest::get(url).await;

    debug!("{} response {:?}", package_name, body);

    if let Ok(response) = body {
        let parsed: Result<AurResults, reqwest::Error> = response.json().await;
        debug!("{} aur_api_query_provides {:?}", package_name, parsed);
        if let Ok(parsed) = parsed {
            for result in parsed.results.iter() {
                if result.name == sanitized_package_name {
                    return Some(AurPackage {
                        package_name: sanitized_package_name,
                        package_base: result.package_base.clone(),
                        repo_deps: Vec::new(),
                    })
                }
            }

            if !strict && !parsed.results.is_empty() {
                return Some(AurPackage {
                    package_name: parsed.results[0].name.clone(),
                    package_base: parsed.results[0].package_base.clone(),
                    repo_deps: Vec::new(),
                })
            }
        }
    }

    warn!("Could not find package {} in query provides", package_name);
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use petgraph::Direction;
    use petgraph::prelude::EdgeRef;
    use serial_test::serial;
    use crate::builder::bubblewrap::Bubblewrap;
    use crate::builder::dependency::{AurPackage, build_dependency_graph};

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