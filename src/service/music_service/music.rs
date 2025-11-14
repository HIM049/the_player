use lofty::{file::TaggedFileExt, tag::Tag};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

/// The music data struct
#[derive(Clone)]
pub struct Music {
    path: PathBuf,
    tags: Option<Tag>,
}

impl Music {
    /// Create a music from path
    pub fn from_path<P>(path: P) -> Result<Self, anyhow::Error>
    where
        P: AsRef<Path>,
    {
        // create music
        let mut music = Self {
            path: path.as_ref().to_path_buf(),
            tags: None,
        };

        // read metadata
        music.read_tags()?;
        Ok(music)
    }

    /// Open music file
    pub fn open_file(&self) -> io::Result<File> {
        File::open(&self.path)
    }

    /// Read tags from file and save to struct
    pub fn read_tags(&mut self) -> Result<(), anyhow::Error> {
        let mut file = self.open_file()?;
        // Try to read music metas
        if let Some(tags) = lofty::read_from(&mut file)?.primary_tag() {
            self.tags = Some(tags.clone());
        }

        Ok(())
    }

    /// Get tags reference of music if exists
    pub fn get_tags(&self) -> Option<&Tag> {
        if let Some(t) = &self.tags {
            return Some(&t);
        }
        None
    }

    /// Get the path reference of music
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
