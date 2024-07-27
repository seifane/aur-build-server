use std::error::Error;
use std::fs;
use std::path::{PathBuf};
use homedir::my_home;
use serde::{Deserialize, Serialize};
use simple_error::SimpleError;

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileConfig {
    default: String,
    profiles: Vec<Profile>,
}

impl ProfileConfig {
    pub fn new() -> Self
    {
        ProfileConfig {
            default: String::new(),
            profiles: Vec::new()
        }
    }

    pub fn get_default_directory() -> PathBuf
    {
        my_home()
            .expect("Unable to get home directory")
            .expect("Unable to get home directory")
            .join(".config")
            .join("aur-build-cli")
    }

    pub fn from_file() -> Result<Self, Box<dyn Error>> {
        let path = Self::get_default_directory().join("profiles.json");
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(content.as_str())?)
    }

    pub fn save_to_file(&self) -> Result<(), Box<dyn Error>>
    {
        let directory = Self::get_default_directory();
        if !directory.exists() {
            fs::create_dir_all(&directory).expect("Failed to create config directory");
        }

        let content = serde_json::to_string(self)?;
        Ok(fs::write(directory.join("profiles.json"), content)?)
    }

    pub fn get_default_profile_name(&self) -> &String {
        &self.default
    }

    pub fn get_default_profile(&self) -> Option<&Profile>
    {
        self.get_profile_by_name(&self.default)
    }

    pub fn set_default_profile(&mut self, name: &String) -> Result<(), Box<dyn Error>>
    {
        if self.get_profile_by_name(name).is_none() {
            return Err(SimpleError::new("Profile does not exist").into());
        }
        self.default = name.to_string();
        Ok(())
    }

    pub fn get_profile_by_name(&self, name: &String) -> Option<&Profile>
    {
        for profile in self.profiles.iter() {
            if &profile.name == name {
                return Some(profile);
            }
        }
        None
    }

    pub fn add_profile(&mut self, profile: Profile) -> Result<(), Box<dyn Error>>
    {
        if self.profiles.is_empty() {
            self.default = profile.name.clone();
        }

        if self.get_profile_by_name(&profile.name).is_some() {
            return Err(SimpleError::new("Profile already exists with this name").into());
        }

        self.profiles.push(profile);
        Ok(())
    }

    pub fn remove_profile(&mut self, name: &String) -> Result<(), Box<dyn Error>>
    {
        if self.get_profile_by_name(name).is_none() {
            return Err(SimpleError::new("No profile exist with that name").into());
        }

        self.profiles = self.profiles.drain(..).filter(|p| &p.name != name).collect();

        Ok(())
    }

    pub fn get_profiles(&self) -> &Vec<Profile>
    {
        &self.profiles
    }
}