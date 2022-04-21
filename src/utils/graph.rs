use std::borrow::{BorrowMut};
use std::fmt::{Display};
use std::slice::{Iter, IterMut};
use crate::utils::package_data::{Package, PackageStatus};

pub struct Node<T> {
    id: usize,
    pub data: T,
}

impl<T> Node<T> where T: Clone {
    pub fn from_data(data: &T) -> Node<T>
    {
        Node {
            id: 0,
            data: data.clone()
        }
    }

    pub fn get_mut(&mut self) -> &mut T
    {
        self.data.borrow_mut()
    }
}

pub struct Graph<T> {
    id_count: usize,
    pub nodes: Vec<Node<T>>,
    adjacency_matrix: Vec<Vec<bool>>
}

impl<T> Graph<T> where T: Clone {
    pub fn new() -> Graph<T>
    {
        Graph {
            id_count: 0,
            nodes: Vec::new(),
            adjacency_matrix: Vec::new()
        }
    }

    pub fn get_node(&self, node_id: usize) -> Option<&'_ Node<T>>
    {
        self.nodes.get(node_id)
    }

    pub fn find_node(&self, f: fn(&T) -> bool) -> Option<usize>
    {
        for (index, item) in self.nodes.iter().enumerate() {
            if f(&item.data) {
                return Some(index)
            }
        }
        None
    }

    pub fn put_node(&mut self, data: &T) -> usize
    {
        self.id_count += 1;

        let mut node = Node::from_data(data);
        node.id = self.id_count;
        self.nodes.push(node);

        self.adjacency_matrix.push(Vec::new());
        for row in self.adjacency_matrix.iter_mut() {
            row.resize(self.nodes.len(), false);
        }

        self.nodes.len() - 1
    }

    pub fn iter(&self) -> Iter<'_, Node<T>>
    {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, Node<T>>
    {
        self.nodes.iter_mut()
    }

    pub fn add_edge(&mut self, src: usize, dest: usize)
    {
        if src > self.nodes.len() || dest > self.nodes.len() {
            return;
        }

        self.adjacency_matrix[src][dest] = true;
    }

    pub fn update_node(&mut self, node_id: usize, data: &T)
    {
        let node = self.nodes.get_mut(node_id).unwrap();
        *node.data.borrow_mut() = data.clone();
    }

    pub fn get_dests(&self, node_id: usize) -> Vec<usize>
    {
        let mut nodes = Vec::new();

        for (index, item) in self.adjacency_matrix[node_id].iter().enumerate() {
            if *item {
                nodes.push(index);
            }
        }

        nodes
    }

    pub fn get_srcs(&self, node_id: usize) -> Vec<usize>
    {
        let mut nodes = Vec::new();

        for (index, item) in self.adjacency_matrix.iter().enumerate() {
            if item[node_id] {
                nodes.push(index);
            }
        }

        nodes
    }
}

pub fn are_dependencies_built(node_id: usize, graph: &Graph<Package>) -> bool
{
    for src in graph.get_srcs(node_id).iter() {
        if graph.get_node(*src).unwrap().data.status != PackageStatus::Built {
            return false;
        }
    }
    true
}

pub fn get_queued_package(graph: &mut Graph<Package>) -> Option<usize> {
    for index in 0..graph.nodes.len() {
        if graph.nodes.get_mut(index).unwrap().data.status != PackageStatus::Queued {
            continue;
        }
        if are_dependencies_built(index, graph) {
            let package = graph.nodes.get_mut(index).unwrap();
            package.data.status = PackageStatus::Building;
            return Some(index);
        }
    }
    None
}

pub fn print_graph_node<T>(node: usize, graph: &Graph<T>) where T: Display + Clone
{
    print!(" -> {}", graph.get_node(node).unwrap().data);
    for dest in graph.get_dests(node) {
        print_graph_node(dest, graph);
        print!("\n");
    }
    print!("\n");
}

pub fn print_graph<T>(graph: &Graph<T>) where T: Clone + Display
{
    for (index, _) in graph.iter().enumerate() {
        if graph.get_srcs(index).len() == 0  {
            print_graph_node(index, graph);
        }
    }
}

#[test]
fn can_add_node()
{
    let mut graph = Graph::new();
    let data = String::from("string");
    let id = graph.put_node(&data);
    assert_eq!(data, graph.get_node(id).unwrap().data);
}

#[test]
fn can_find_node()
{
    let mut graph = Graph::new();
    graph.put_node(&String::from("string"));
    graph.put_node(&String::from("string1"));
    graph.put_node(&String::from("string2"));
    graph.put_node(&String::from("string3"));
    graph.put_node(&String::from("string4"));

    let res = graph.find_node(|i| {
        return i.as_str() == "string3"
    });

    assert_eq!(true, res.is_some());
    assert_eq!("string3", graph.get_node(res.unwrap()).unwrap().data.as_str());

    let res_none = graph.find_node(|i| {
        return i.as_str() == "string5"
    });

    assert_eq!(true, res_none.is_none());
}

#[test]
fn can_add_edges() {
    let mut graph = Graph::new();
    let n_l = graph.put_node(&String::from("league"));
    let n_w = graph.put_node(&String::from("wine"));
    let n_s = graph.put_node(&String::from("something"));

    graph.add_edge(n_w, n_l);
    graph.add_edge(n_s, n_l);

    assert_eq!(1, graph.get_dests(n_w).len());
    assert_eq!(n_l, graph.get_dests(n_w)[0]);

    assert_eq!(1, graph.get_dests(n_s).len());
    assert_eq!(n_l, graph.get_dests(n_s)[0]);

    assert_eq!(2, graph.get_srcs(n_l).len());
    assert_eq!(vec![n_w, n_s], graph.get_srcs(n_l));
}