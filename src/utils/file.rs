use std::fs::File;
use std::io;
use std::io::Read;

pub fn read_file_to_string(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}