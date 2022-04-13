use indextree::{Arena, Node, NodeId};
use crate::utils::package::{filter_aur_deps, get_dependencies};
use crate::utils::package_data::{Package, PackageStatus};

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
        if package.get().status == PackageStatus::Queued ||
            package.get().status == PackageStatus::QueuedForce {

            if package.parent().is_some() {
                let parent_node = arena.get(package.parent().unwrap()).unwrap();
                if parent_node.get().status != PackageStatus::Built {
                    continue;
                }
            }

            return Some(arena.get_node_id(package).unwrap());
        }
    }
    None
}

pub fn insert_dependency(arena: &mut Arena<Package>, dep: &Package, of_package: &NodeId) {
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

            let aur_deps = filter_aur_deps(get_dependencies(package, false));
            for aur_dep in aur_deps.iter() {
                insert_dependency(
                    arena,
                    &Package::from_package_name(aur_dep),
                    &package_node
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

pub fn print_tree(arena: &Arena<Package>) {
    for node in arena.iter() {
        if node.parent().is_none() {
            print_node(arena, node, 0);
        }
    }
}