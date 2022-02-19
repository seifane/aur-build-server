use std::time::SystemTime;
use indextree::{Arena, Node, NodeId};
use serde::Serialize;

use crate::utils::package::{filter_aur_deps, get_dependencies};
use crate::utils::package_data::PackageStatus::{Queued, QueuedForce};

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
    pub last_build_commit: Option<String>,
    pub time: SystemTime,
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Package {
    pub fn from_package_name(package_name: &String) -> Package {
        Package {
            name: package_name.clone(),
            run_before: None,
            status: Queued,
            last_build_commit: None,
            time: SystemTime::now()
        }
    }
}

pub fn get_by_package_name(arena: &Arena<Package>, package_name: &String) -> Option<NodeId> {
    for package in arena.iter() {
        if package.get().name.eq(package_name)  {
            return Some(arena.get_node_id(package).unwrap())
        }
    }
    None
}

pub fn get_queued_branch(arena: &Arena<Package>) -> Option<NodeId> {
    for package in arena.iter() {
        if package.parent().is_none() &&
            (package.get().status == Queued || package.get().status == QueuedForce) {
            return Some(arena.get_node_id(package).unwrap());
        }
    }
    None
}

pub fn insert_dependence(dep: &Package, of_package: &NodeId, arena: &mut Arena<Package>) {
    let existing = get_by_package_name(arena, &dep.name);
    match existing {
        None => {
            let dep_node = arena.new_node(dep.clone());
            dep_node.append(*of_package, arena);
        }
        Some(dep_node) => {
            dep_node.append(*of_package, arena);
        }
    }
}

pub fn insert_package(package: &Package, arena: &mut Arena<Package>) {
    let node = get_by_package_name(arena, &package.name);

    match node {
        None => {
            let package_node = arena.new_node(package.clone());

            let aur_deps = filter_aur_deps(get_dependencies(package));
            for aur_dep in aur_deps.iter() {
                insert_dependence(
                    &Package::from_package_name(aur_dep),
                    &package_node,
                    arena
                );
            }
        },
        Some(node) => {
            let package_node = arena.get_mut(node).unwrap();
            *package_node.get_mut() = package.clone();
        }
    }
}

fn print_node(arena: &Arena<Package>, node: &Node<Package>, level: u8) {
    for _ in 0..(level + 1) * 2 {
        print!("-");
    }
    print!(" {}\n", node.get().name);

    let mut child = node.first_child();
    while child.is_some() {
        let node = arena.get(child.unwrap());
        if node.is_some() {
            print_node(arena, node.unwrap(), level + 1);
            child = node.unwrap().next_sibling();
        } else {
            child = None
        }
    }
}

pub fn print_dep_tree(arena: &Arena<Package>) {
    for node in arena.iter() {
        if node.parent().is_none() {
            print_node(arena, node, 0);
        }
    }
}