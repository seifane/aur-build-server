use std::borrow::{Borrow, BorrowMut};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::error::Error;
use std::time::{Duration, SystemTime};
use indextree::{Arena, NodeId};
use crate::config::Config;
use crate::utils::{build_repo};
use crate::utils::package::{copy_package_to_repo, make_package};
use crate::utils::package_data::{Package, PackageStatus};
use crate::utils::tree::{get_queued_branch, insert_package, print_tree};


#[derive(Clone)]
pub struct PackageManager {
    pub is_running: Arc<AtomicBool>,
    pub commit_queued: Arc<AtomicBool>,
    pub dependency_lock: Arc<(Mutex<bool>, Condvar)>,
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
            package_tree: Arc::new(Mutex::new(Arena::new())),
            workers_handles: Arc::new(Mutex::new(vec![])),
            config,
        }
    }

    pub fn worker_thread(&mut self) {
        info!("Starting worker thread");
        while self.is_running.load(Ordering::SeqCst) {
            let mut tree = self.package_tree.lock().unwrap();
            let branch: Option<NodeId> = get_queued_branch(tree.borrow());

            match branch {
                None => {
                    drop(tree);
                    thread::sleep(Duration::from_millis(1000));
                },
                Some(node_id) => {
                    let node = tree.get_mut(node_id).unwrap().get_mut();
                    node.status = PackageStatus::Building;
                    drop(tree);
                    self.process_package_node(node_id);
                }
            }
        }
        info!("Stopping worker thread");
    }

    fn process_package_node(&mut self, node_id: NodeId) {
        let mut node = self.package_tree.lock().unwrap().get(node_id).unwrap().get().clone();
        let mut child = self.package_tree.lock().unwrap().get(node_id).unwrap().first_child().clone();

        self.process_package(&mut node, child.is_some());

        while child.is_some() {
            let arena = self.package_tree.lock().unwrap();

            let child_id = child.unwrap();
            let node = arena.get(child_id).unwrap();
            let do_install = node.first_child().is_some();
            let mut package = node.get().clone();
            child = node.next_sibling();
            drop(arena);

            self.process_package(&mut package, do_install);
        }
    }

    fn process_package(&mut self, package: &mut Package, do_install: bool) {
        let force = package.status == PackageStatus::QueuedForce;
        package.status = PackageStatus::Building;
        insert_package(package, self.package_tree.lock().unwrap().borrow_mut());

        info!("Making package {} install {}", package.name, do_install);

        let res = make_package(package, self.dependency_lock.clone(), force);

        package.status = res.is_ok().then(|| PackageStatus::Built).unwrap_or(PackageStatus::Failed);

        insert_package(package, self.package_tree.lock().unwrap().borrow_mut());
        if package.status == PackageStatus::Built {
            info!("Built package {}", package.name);
            copy_package_to_repo(&package.name).unwrap();
        } else {
            error!("Failed to build package {}", package.name);
        }

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

        self.config.packages.iter().for_each(|package_config| {
            let package = Package {
                name: package_config.name.clone(),
                run_before: package_config.run_before.clone(),
                status: PackageStatus::Queued,
                last_build_commit: None,
                time: SystemTime::now(),
            };
            insert_package(&package, self.package_tree.lock().unwrap().borrow_mut());
        });
        println!("Loaded packages");
        print_tree(self.package_tree.lock().unwrap().borrow());
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
                if package.status != PackageStatus::Built {
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