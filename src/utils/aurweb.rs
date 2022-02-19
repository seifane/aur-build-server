use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageData {
    #[serde(rename = "Depends")]
    pub depends: Option<Vec<String>>,
    #[serde(rename = "MakeDepends")]
    pub make_depends: Option<Vec<String>>,
    #[serde(rename = "OptDepends")]
    pub opt_depends: Option<Vec<String>>,
    #[serde(rename = "CheckDepends")]
    pub check_depends: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageDataResponse {
    #[serde(rename = "resultcount")]
    pub result_count: i64,
    pub results: Vec<PackageData>,
    #[serde(rename = "type")]
    pub response_type: String,
    pub version: i64
}

pub fn get_package_data(package_name: &String) -> reqwest::Result<PackageDataResponse> {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg[]={}", package_name);
    debug!("Getting AUR package data for {}", package_name);
    reqwest::blocking::get(
        url
    )?.json::<PackageDataResponse>()
}