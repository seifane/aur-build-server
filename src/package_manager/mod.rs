use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::error::Error;
use std::time::{Duration, SystemTime};
use serde::Serialize;
use crate::config::Config;
use crate::utils::{build_repo, copy_package_to_repo, make_package};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PackageStatus {
    QUEUED,
    QUEUED_FORCE,
    BUILDING,
    BUILT,
    FAILED,
}

#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub time: SystemTime,
}

pub struct PackageManager {
    pub is_running: Arc<AtomicBool>,
    pub commit_queued: Arc<AtomicBool>,
    pub dependency_lock: Arc<(Mutex<bool>, Condvar)>,
    pub packages: Arc<Mutex<Vec<Package>>>,
    workers_handles: Vec<JoinHandle<()>>,
    config: Config,
}

impl PackageManager {
    pub fn new(config: Config) -> PackageManager {
        PackageManager {
            is_running: Arc::new(AtomicBool::new(false)),
            commit_queued: Arc::new(AtomicBool::new(false)),
            dependency_lock: Arc::new((Mutex::new(false), Condvar::new())),
            packages: Arc::new(Mutex::new(Vec::new())),
            workers_handles: vec![],
            config,
        }
    }

    pub fn start_workers(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);

        for _ in 0..5 {
            let packages = self.packages.clone();
            let is_running = self.is_running.clone();
            let dependency_lock = self.dependency_lock.clone();

            self.workers_handles.push(thread::spawn(move || {
                println!("Starting worker thread");
                while is_running.load(Ordering::SeqCst) {
                    let mut package = None;
                    let mut force = false;
                    {
                        let mut locked_packages = packages.lock().unwrap();
                        let queue_package = locked_packages
                            .iter_mut()
                            .filter(|i|
                                i.status == PackageStatus::QUEUED || i.status == PackageStatus::QUEUED_FORCE
                            ).next();

                        if queue_package.is_some() {
                            let mut pkg = queue_package.unwrap();
                            force = pkg.status == PackageStatus::QUEUED_FORCE;
                            package = Some(pkg.clone());
                            pkg.status = PackageStatus::BUILDING;
                        }
                    }

                    match package {
                        None => { thread::sleep(Duration::from_millis(1000)); }
                        Some(package) => {
                            println!("Making package {}", package.name);

                            let res = make_package(&package, Arc::clone(&dependency_lock), force);

                            let mut locked_packages = packages.lock().unwrap();
                            let queue_package = locked_packages
                                .iter_mut()
                                .filter(|i| i.name == package.name).next();
                            if queue_package.is_some() {
                                let mut pkg = queue_package.unwrap();
                                pkg.status = res.is_ok().then(|| PackageStatus::BUILT).unwrap_or(PackageStatus::FAILED);
                                if pkg.status == PackageStatus::BUILT {
                                    copy_package_to_repo(pkg.name.clone()).unwrap();
                                }
                            }

                            println!("Built package {}", package.name);
                        }
                    }
                }
                println!("Stopping worker thread");
            }));
        }
    }

    pub fn stop_workers(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        // TODO : Join workers
        // TODO : stop
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
                    status: PackageStatus::QUEUED,
                    time: SystemTime::now(),
                }
            )
        });
    }

    pub fn rebuild_packages(&mut self) {
        self.packages.lock().unwrap().iter_mut().for_each(|package| {
            if package.status == PackageStatus::BUILT {
                package.status = PackageStatus::QUEUED;
            }
        });
    }

    pub fn rebuild_package(&mut self, package_name: String, force: bool) {
        self.packages.lock().unwrap().iter_mut().for_each(|package| {
            if package.name == package_name &&
                (package.status == PackageStatus::BUILT || package.status == PackageStatus::FAILED) {
                package.status = if force {PackageStatus::QUEUED_FORCE} else {PackageStatus::QUEUED}
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
                // println!("Waiting for packages to be built to commit ...");
                thread::sleep(Duration::from_secs(3));
            }

            println!("Committing packages to repo ...");
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
        i.status == PackageStatus::BUILDING ||
        i.status == PackageStatus::QUEUED ||
        i.status == PackageStatus::QUEUED_FORCE
    }).count()
}