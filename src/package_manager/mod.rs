use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::error::Error;
use std::time::{Duration, SystemTime};
use serde::Serialize;
use crate::config::Config;
use crate::utils::{build_repo};
use crate::utils::package::{copy_package_to_repo, make_package};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PackageStatus {
    Queued,
    QueuedForce,
    Building,
    Built,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub dependency_for: Vec<String>,
    pub time: SystemTime,
}

#[derive(Clone)]
pub struct PackageManager {
    pub is_running: Arc<AtomicBool>,
    pub commit_queued: Arc<AtomicBool>,
    pub dependency_lock: Arc<(Mutex<bool>, Condvar)>,
    pub packages: Arc<Mutex<Vec<Package>>>,
    workers_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    config: Config,
}

impl PackageManager {
    pub fn new(config: Config) -> PackageManager {
        PackageManager {
            is_running: Arc::new(AtomicBool::new(false)),
            commit_queued: Arc::new(AtomicBool::new(false)),
            dependency_lock: Arc::new((Mutex::new(false), Condvar::new())),
            packages: Arc::new(Mutex::new(Vec::new())),
            workers_handles: Arc::new(Mutex::new(vec![])),
            config,
        }
    }

    pub fn cron_thread(packages: Arc<Mutex<Vec<Package>>>, config: Config) {
        if config.rebuild_time.is_none() {
            info!("Not starting automatic refresh ...");
            return;
        }
        loop {
            std::thread::sleep(Duration::from_secs(config.rebuild_time.unwrap()));
            info!("Rebuilding packages ...");
            packages.lock().unwrap().iter_mut().for_each(|package| {
                if package.status == PackageStatus::Built {
                    package.status = PackageStatus::Queued;
                }
            });
        }
    }

    pub fn start_cron_thread(&mut self) {
        let packages = self.packages.clone();
        let config = self.config.clone();
        std::thread::spawn(|| {
            PackageManager::cron_thread(packages, config);
        });
    }

    fn pop_package_from_queue(&mut self) -> Option<Package> {
        let mut locked_packages = self.packages.lock().unwrap();
        // Ideally sorting should only be done when inserting or updating
        locked_packages.sort_by(|a, b| {
            a.dependency_for.len().partial_cmp(&b.dependency_for.len()).unwrap()
        });
        let queue_package = locked_packages
            .iter_mut()
            .filter(|i|
                i.status == PackageStatus::Queued || i.status == PackageStatus::QueuedForce
            ).next();
        match queue_package {
            None => None,
            Some(package) => Some(package.clone())
        }
    }


    //TODO: handle inserts
    fn push_package(&mut self, package: &Package) {
        let mut locked_packages = self.packages.lock().unwrap();
        let queue_package = locked_packages
            .iter_mut()
            .filter(|i| i.name == package.name).next();
        if queue_package.is_some() {
            let pak = queue_package.unwrap();
            pak.dependency_for = package.dependency_for.clone();
            pak.status = package.status.clone();
            pak.time = package.time.clone();
            pak.run_before = package.run_before.clone();
        }
    }

    pub fn worker_thread(&mut self) {
        while self.is_running.load(Ordering::SeqCst) {
            let queue_package = self.pop_package_from_queue();
            if queue_package.is_none() {
                thread::sleep(Duration::from_millis(1000));
            }

            let mut package = queue_package.unwrap();
            let mut force = false;

            force = package.status == PackageStatus::QueuedForce;
            package.status = PackageStatus::Building;
            self.push_package(&package);

            info!("Making package {}", package.name);

            let res = make_package(&package, self.dependency_lock.clone(), force);

            package.status = res.is_ok().then(|| PackageStatus::Built).unwrap_or(PackageStatus::Failed);

            self.push_package(&package);
            if package.status == PackageStatus::Built {
                copy_package_to_repo(&package.name).unwrap();
            }

            info!("Built package {}", package.name);
        }
    }

    pub fn start_workers(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);

        for _ in 0..5 {
            let mut cloned_self = self.clone();
            thread::spawn(move || cloned_self.worker_thread());

            /*self.workers_handles.push(thread::spawn(move || {
                info!("Starting worker thread");
                PackageManager::worker_thread(packages, is_running, dependency_lock);
                info!("Stopping worker thread");
            }));*/
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
        if self.packages.lock().unwrap().len() > 0 {
            return;
        }

        self.config.packages.iter().for_each(|package_config| {
            self.packages.lock().unwrap().push(
                Package {
                    name: package_config.name.clone(),
                    run_before: package_config.run_before.clone(),
                    status: PackageStatus::Queued,
                    time: SystemTime::now(),
                    dependency_for: vec![],
                }
            )
        });
    }

    pub fn rebuild_packages(&mut self) {
        self.packages.lock().unwrap().iter_mut().for_each(|package| {
            if package.status == PackageStatus::Built {
                package.status = PackageStatus::Queued;
            }
        });
    }

    pub fn rebuild_package(&mut self, package_name: String, force: bool) {
        self.packages.lock().unwrap().iter_mut().for_each(|package| {
            if package.name == package_name &&
                (package.status == PackageStatus::Built || package.status == PackageStatus::Failed) {
                package.status = if force { PackageStatus::QueuedForce } else { PackageStatus::Queued }
            }
        });
    }

    pub fn queue_commit(&mut self) {
        if self.commit_queued.load(Ordering::SeqCst) {
            return;
        }

        let packages = self.packages.clone();
        let commit_queued = self.commit_queued.clone();
        let is_running = self.is_running.clone();
        let repo_name = self.config.repo_name.clone().unwrap_or(String::from("aurbuild"));

        thread::spawn(move || {
            commit_queued.store(true, Ordering::SeqCst);

            while get_pending_packages_count(packages.clone()) != 0
                && is_running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(3));
            }

            info!("Committing packages to repo ...");
            build_repo(repo_name).unwrap(); //TODO: handle
            commit_queued.store(false, Ordering::SeqCst);
        });
    }


    pub fn commit_now(&mut self) -> Result<(), Box<dyn Error>> {
        build_repo(self.config.repo_name.clone().unwrap_or(String::from("aurbuild")))?;
        Ok(())
    }
}

fn get_pending_packages_count(packages: Arc<Mutex<Vec<Package>>>) -> usize {
    packages.lock().unwrap().iter().filter(|i| {
        i.status == PackageStatus::Building ||
            i.status == PackageStatus::Queued ||
            i.status == PackageStatus::QueuedForce
    }).count()
}