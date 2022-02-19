use std::io;
use regex::Regex;
use crate::utils::file::read_file_to_string;
use crate::utils::package_data::Package;

pub fn read_dependencies(package: &Package, dependency_type: &str) -> Result<Vec<String>, io::Error> {
    let path = format!("data/{}/PKGBUILD", package.name);
    let pkgbuild = read_file_to_string(path.as_str()).unwrap();

    let mut deps = vec![];

    let search = format!("{}=(", dependency_type);
    let found_opt = pkgbuild.find(&search);
    if found_opt.is_none() {
        return Ok(deps);
    }
    let found = found_opt.unwrap();
    let found_end = pkgbuild.get(found..).unwrap().find(")").map(|i| i + found).unwrap();

    let depends = pkgbuild.get(found..found_end).unwrap();

    let re = Regex::new(r"'([^']+)'").unwrap();
    for cap in re.captures_iter(depends) {
        deps.push(String::from(cap.get(1).unwrap().as_str()));
    }
    Ok(deps)
}

pub fn parse_opt_deps(depends: Vec<String>) -> Vec<String> {
    let mut parsed: Vec<String> = Vec::new();

    for item in depends.iter() {
        let mut split = item.split(':');
        let package_name = split.next();
        if package_name.is_some()  {
            parsed.push(package_name.unwrap().to_string());
        }
    }

    parsed
}