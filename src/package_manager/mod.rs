use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use serde::Serialize;
use crate::config::Config;
use crate::utils::{copy_packages, make_package};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PackageStatus {
    QUEUED,
    BUILDING,
    BUILT,
    FAILED
}

#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub name: String,
    pub status: PackageStatus,
    pub time: SystemTime,
}

pub struct PackageManager {
    pub is_running: Arc<AtomicBool>,
    pub commit_queued: Arc<AtomicBool>,
    pub packages: Arc<Mutex<Vec<Package>>>,
    workers_handles: Vec<JoinHandle<()>>,
    config: Config,
}

impl PackageManager {
    pub fn new(config: Config) -> PackageManager {
        PackageManager {
            is_running: Arc::new(AtomicBool::new(false)),
            commit_queued: Arc::new(AtomicBool::new(false)),
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

            self.workers_handles.push(thread::spawn(move || {
                println!("Starting worker thread");
                while is_running.load(Ordering::SeqCst) {
                    let mut package = None;
                    {
                        let mut locked_packages = packages.lock().unwrap();
                        let queue_package = locked_packages
                            .iter_mut()
                            .filter(|i| i.status == PackageStatus::QUEUED).next();

                        if queue_package.is_some() {
                            let mut pkg = queue_package.unwrap();
                            package = Some(pkg.name.clone());
                            pkg.status = PackageStatus::BUILDING;
                        }
                    }

                    match package {
                        None => {thread::sleep(Duration::from_millis(1000));}
                        Some(package_name) => {
                            println!("Making package {}", package_name);

                            let res = make_package(package_name.clone());

                            let mut locked_packages = packages.lock().unwrap();
                            let queue_package = locked_packages
                                .iter_mut()
                                .filter(|i| i.name == package_name).next();
                            if queue_package.is_some() {
                                let mut pkg = queue_package.unwrap();
                                pkg.status = res.is_ok().then(|| PackageStatus::BUILT).unwrap_or(PackageStatus::FAILED);
                            }

                            println!("Built package {}", package_name);
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

        self.config.packages.iter().for_each(|package_name| {
           self.packages.lock().unwrap().push(
               Package {
                   name: package_name.clone(),
                   status: PackageStatus::QUEUED,
                   time: SystemTime::now()
               }
           )
        });
    }

    pub fn rebuild_packages(&mut self) {
        self.packages.lock().unwrap().iter_mut().for_each(|package|{
            if package.status == PackageStatus::BUILT {
                package.status = PackageStatus::QUEUED;
            }
        });
    }

    pub fn rebuild_package(&mut self, package_name: String) {
        self.packages.lock().unwrap().iter_mut().for_each(|package| {
            if package.name == package_name &&
                (package.status == PackageStatus::BUILT || package.status == PackageStatus::FAILED) {
                package.status = PackageStatus::QUEUED
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
        thread::spawn(move || {
            commit_queued.store(true, Ordering::SeqCst);

            while packages.lock().unwrap().iter().filter(|i| i.status != PackageStatus::BUILT).count() != 0
                && is_running.load(Ordering::SeqCst) {
                println!("Waiting for packages to be built to commit ...");
                thread::sleep(Duration::from_secs(3));
            }

            println!("Committing packages to repo ...");
            copy_packages();
            commit_queued.store(false, Ordering::SeqCst);
        });
    }

}