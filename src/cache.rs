use std::path::PathBuf;
use sha2::{Sha256, Digest};

pub struct Cache {
    pub dir: PathBuf,
}

impl Cache {
    pub fn new(dir: &str) -> Self {
        std::fs::create_dir_all(dir).expect("Could not create cache dir");
        Cache { dir: PathBuf::from(dir) }
    }

    pub fn key(&self, url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn path(&self, key: &str) -> PathBuf {
        self.dir.join(key)
    }

    pub fn exists(&self, key: &str) -> bool {
        self.path(key).exists()
    }

    pub fn link(&self, key: &str, out_path: &str) -> anyhow::Result<()> {
        let src = self.path(key);
        if std::path::Path::new(out_path).exists() {
            std::fs::remove_file(out_path)?;
        }
        std::fs::hard_link(&src, out_path)?;
        println!("Cache hit - hard-linked {} to {}", src.display(), out_path);
        Ok(())
    }

    pub fn store(&self, key: &str, out_path: &str) -> anyhow::Result<()> {
        let dest = self.path(key);
        std::fs::rename(out_path, &dest)?;
        std::fs::hard_link(&dest, out_path)?;
        println!("Stored in cache: {}", dest.display());
        Ok(())
    }
}