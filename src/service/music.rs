use std::{fs::File, path::{Path, PathBuf}};

use lofty::{file::TaggedFileExt, tag::Tag};

#[derive(Clone)]
pub struct Music {
    path: PathBuf,
    tags: Option<Tag>
}

impl Music {
    pub fn from_path<P>(path: P) -> Result<Self, anyhow::Error> 
    where P: AsRef<Path>
    {
        let mut file = File::open(&path)?;
        let mut music_tag: Option<Tag> = None;
        if let Some(tags) = lofty::read_from(&mut file)?.primary_tag() {
            music_tag = Some(tags.clone());
        }

        Ok(
            Self { 
                path: path.as_ref().to_path_buf(), 
                tags: music_tag
            }
        )
    }

    pub fn open_file(&self) -> Result<File, std::io::Error> {
        File::open(&self.path)
    }

    pub fn get_tags(&self) -> Option<&Tag>{
        if let Some(t) = &self.tags {
            Some(&t)
        } else {
            None
        }
    }
}