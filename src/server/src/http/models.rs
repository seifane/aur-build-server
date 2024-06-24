use crate::http::multipart::MultipartField;

pub struct PackageBuildData<'a> {
    pub files: Option<&'a Vec<MultipartField>>,
    pub log_files: Option<&'a Vec<MultipartField>>,

    pub errors: Vec<String>,
    pub version: Option<String>,
}