use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::error::Error;
use std::time::{Duration, SystemTime};
use indextree::{Arena};
use crate::config::Config;
use crate::utils::{build_repo, makepkg, pacman};
use crate::utils::git::clone_repo;
use crate::utils::graph::{get_queued_package, Graph, print_graph};
use crate::utils::package::{copy_package_to_repo, make_package};
use crate::utils::package_data::{Package, PackageStatus};


#[derive(Clone)]
pub struct PackageManager {
    pub is_running: Arc<AtomicBool>,
    pub commit_queued: Arc<AtomicBool>,
    pub dependency_lock: Arc<(Mutex<bool>, Condvar)>,
    pub package_graph: Arc<Mutex<Graph<Package>>>,
    pub package_tree: Arc<Mutex<Arena<Package>>>,
    workers_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    config: Config,
}

impl PackageManager {
    pub fn new(config: Config) -> PackageManager {
        PackageManager {
            is_running: Arc::new(AtomicBool::new(false)),
            commit_queued: Arc::new(AtomicBool::new(false)),
            dependency_lock: Arc::new((Mutex::new(false), Condvar::new())),
            package_graph: Arc::new(Mutex::new(Graph::new())),
            package_tree: Arc::new(Mutex::new(Arena::new())),
            workers_handles: Arc::new(Mutex::new(vec![])),
            config,
        }
    }

    pub fn worker_thread(&mut self) {
        info!("Starting worker thread");
        while self.is_running.load(Ordering::SeqCst) {
            let node_id = get_queued_package(self.package_graph.lock().unwrap().borrow_mut());

            match node_id {
                None => {
                    thread::sleep(Duration::from_millis(1000));
                }
                Some(node_id) => {
                    self.process_package(node_id);
                }
            }
        }
        info!("Stopping worker thread");
    }

    fn process_package(&mut self, node_id: usize) -> bool {
        let mut package = self.package_graph.lock().unwrap().get_node(node_id).unwrap().data.clone();
        let do_install = self.package_graph.lock().unwrap().get_dests(node_id).len() != 0;

        package.status = PackageStatus::Building;
        self.package_graph.lock().unwrap().update_node(node_id, &package);

        info!("Making package {} install {}", package.name, do_install);

        let res = make_package(&mut package, self.dependency_lock.clone());

        package.status = res.is_ok().then(|| PackageStatus::Built).unwrap_or(PackageStatus::Failed);

        self.package_graph.lock().unwrap().update_node(node_id, &package);
        if package.status == PackageStatus::Built {
            info!("Built package {}", package.name);
            copy_package_to_repo(&package.name).unwrap();
            build_repo(self.config.repo_name.clone().unwrap_or(String::from("aurbuild"))).unwrap_or(());
            return true;
        }
        error!("Failed to build package {}", package.name);
        false
    }


    pub fn start_workers(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);

        let workers_count = self.config.workers_count.unwrap_or(5);

        info!("Spawning {} workers", workers_count);

        for _ in 0..workers_count {
            let mut cloned_self = self.clone();
            self.workers_handles.lock().unwrap().push(
                thread::spawn(move || cloned_self.worker_thread())
            );
        }
    }

    pub fn stop_workers(&mut self) {
        info!("Stopping all worker threads");

        self.is_running.store(false, Ordering::SeqCst);

        while let Some(thread) = self.workers_handles.lock().unwrap().pop() {
            thread.join().unwrap();
        }

        info!("Stopped all worker threads");
    }

    pub fn load_packages(&mut self) {
        if !self.package_tree.lock().unwrap().is_empty() {
            return;
        }

        info!("Loading packages ...");

        self.config.packages.iter().for_each(|package_config| {
            let commit_id = clone_repo(&package_config.name);
            let package = Package {
                name: package_config.name.clone(),
                run_before: package_config.run_before.clone(),
                status: PackageStatus::Queued,
                last_build_commit: if commit_id.is_ok() {Some(commit_id.unwrap())} else {None},
                time: SystemTime::now(),
            };
            self.package_graph.lock().unwrap().put_node(&package);
        });

        self.load_dependencies();

        info!("Loaded packages");
        print_graph(self.package_graph.lock().unwrap().borrow());
    }

    pub fn load_dependencies(&mut self) {
        let mut packages = HashMap::new();
        for (index, package) in self.package_graph.lock().unwrap().iter().enumerate() {
            packages.insert(index, package.data.name.clone());
        }

        while !packages.is_empty() {
            let mut next_deps = HashMap::new();
            for (index, package_name) in packages.iter() {

                let deps = makepkg::get_dependencies(package_name);
                let (_, aur_deps) = pacman::split_repo_aur_packages(deps);

                for aur_dep in aur_deps.iter() {
                    let mut graph_lock = self.package_graph.lock().unwrap();

                    let node_id = graph_lock.put_node(&Package::from_package_name(aur_dep));
                    graph_lock.add_edge(node_id, *index);

                    next_deps.insert(node_id, aur_dep.clone());
                }
            }
            packages.clear();
            packages = next_deps;
        }
    }

    pub fn rebuild_packages(&mut self) {
        let mut tree = self.package_tree.lock().unwrap();

        for node in tree.iter_mut() {
            let package = node.get_mut();
            if package.status == PackageStatus::Built {
                package.status = PackageStatus::Queued;
            }
        }
    }

    pub fn rebuild_package(&mut self, package_name: String, force: bool) {
        let mut tree = self.package_tree.lock().unwrap();

        for node in tree.iter_mut() {
            let package = node.get_mut();
            if package.name == package_name {
                if package.status == PackageStatus::Building {
                    warn!("Not rebuilding package {} because it's not built yet", package.name);
                    return;
                }
                package.status = if force { PackageStatus::QueuedForce } else { PackageStatus::Queued };
            }
        }
    }

    pub fn commit(&mut self) -> Result<(), Box<dyn Error>> {
        build_repo(self.config.repo_name.clone().unwrap_or(String::from("aurbuild")))?;
        Ok(())
    }
}