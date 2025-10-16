use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for file storage
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base directory for uploaded files
    pub upload_dir: PathBuf,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Allowed file extensions (None = allow all)
    pub allowed_extensions: Option<Vec<String>>,
    /// Whether to preserve original filenames
    pub preserve_filenames: bool,
    /// Whether to create subdirectories by date
    pub use_date_subdirs: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            upload_dir: PathBuf::from("./uploads"),
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_extensions: None,
            preserve_filenames: true,
            use_date_subdirs: false,
        }
    }
}

/// Information about an uploaded file
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename from the client
    pub original_filename: Option<String>,
    /// Stored filename on disk
    pub stored_filename: String,
    /// Full path to the stored file
    pub file_path: PathBuf,
    /// File size in bytes
    pub size: usize,
    /// Content type if provided
    pub content_type: Option<String>,
    /// Upload timestamp
    pub upload_time: SystemTime,
}

/// File storage manager
#[derive(Debug, Clone)]
pub struct FileStorage {
    config: StorageConfig,
}

impl FileStorage {
    pub fn new(config: StorageConfig) -> io::Result<Self> {
        // Create upload directory if it doesn't exist
        if !config.upload_dir.exists() {
            fs::create_dir_all(&config.upload_dir)?;
        }
        
        Ok(FileStorage { config })
    }
    
    /// Store uploaded file data
    pub fn store_file(
        &self,
        data: &[u8],
        original_filename: Option<String>,
        content_type: Option<String>,
    ) -> io::Result<UploadedFile> {
        // Validate file size
        if data.len() > self.config.max_file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("File size {} exceeds maximum {}", data.len(), self.config.max_file_size)
            ));
        }
        
        // Validate file extension if restrictions are configured
        if let Some(ref allowed_exts) = self.config.allowed_extensions {
            if let Some(ref filename) = original_filename {
                let extension = Path::new(filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                if !allowed_exts.iter().any(|ext| ext.to_lowercase() == extension) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("File extension '{}' not allowed", extension)
                    ));
                }
            }
        }
        
        // Determine storage directory
        let storage_dir = if self.config.use_date_subdirs {
            let now = SystemTime::now();
            let date_str = format!("{}", now.duration_since(UNIX_EPOCH).unwrap().as_secs() / 86400);
            self.config.upload_dir.join(date_str)
        } else {
            self.config.upload_dir.clone()
        };
        
        // Create storage directory if needed
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }
        
        // Generate stored filename
        let stored_filename = if self.config.preserve_filenames {
            if let Some(ref original) = original_filename {
                self.generate_safe_filename(original, &storage_dir)?
            } else {
                self.generate_random_filename(content_type.as_deref())?
            }
        } else {
            self.generate_random_filename(content_type.as_deref())?
        };
        
        let file_path = storage_dir.join(&stored_filename);
        
        // Write file data
        let mut file = File::create(&file_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        
        Ok(UploadedFile {
            original_filename,
            stored_filename,
            file_path,
            size: data.len(),
            content_type,
            upload_time: SystemTime::now(),
        })
    }
    
    /// Generate a safe filename that doesn't conflict with existing files
    fn generate_safe_filename(&self, original: &str, dir: &Path) -> io::Result<String> {
        // Sanitize the filename
        let sanitized = self.sanitize_filename(original);
        
        // Check if file already exists and add counter if needed
        let mut counter = 0;
        let mut filename = sanitized.clone();
        
        while dir.join(&filename).exists() {
            counter += 1;
            
            // Split filename and extension
            if let Some(dot_pos) = sanitized.rfind('.') {
                let name = &sanitized[..dot_pos];
                let ext = &sanitized[dot_pos..];
                filename = format!("{}_{}{}", name, counter, ext);
            } else {
                filename = format!("{}_{}", sanitized, counter);
            }
        }
        
        Ok(filename)
    }
    
    /// Generate a random filename
    fn generate_random_filename(&self, content_type: Option<&str>) -> io::Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        
        // Try to determine extension from content type
        let extension = content_type
            .and_then(|ct| self.extension_from_content_type(ct))
            .unwrap_or_else(|| "bin".to_string());
        
        Ok(format!("upload_{}_{}.{}", timestamp, rand::random::<u32>(), extension))
    }
    
    /// Sanitize filename to remove dangerous characters
    fn sanitize_filename(&self, filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                // Replace dangerous characters with underscores
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                // Keep safe characters
                c if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' => c,
                // Replace other characters with underscores
                _ => '_',
            })
            .collect::<String>()
            .trim_matches('.')  // Remove leading/trailing dots
            .to_string()
    }
    
    /// Get file extension from content type
    fn extension_from_content_type(&self, content_type: &str) -> Option<String> {
        match content_type.to_lowercase().as_str() {
            "image/jpeg" => Some("jpg".to_string()),
            "image/png" => Some("png".to_string()),
            "image/gif" => Some("gif".to_string()),
            "image/webp" => Some("webp".to_string()),
            "text/plain" => Some("txt".to_string()),
            "text/html" => Some("html".to_string()),
            "text/css" => Some("css".to_string()),
            "text/javascript" => Some("js".to_string()),
            "application/json" => Some("json".to_string()),
            "application/pdf" => Some("pdf".to_string()),
            "application/zip" => Some("zip".to_string()),
            _ => None,
        }
    }
    
    /// Delete a stored file
    pub fn delete_file(&self, file_path: &Path) -> io::Result<()> {
        // Ensure the file is within our upload directory for security
        let canonical_upload_dir = self.config.upload_dir.canonicalize()?;
        let canonical_file_path = file_path.canonicalize()?;
        
        if !canonical_file_path.starts_with(&canonical_upload_dir) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "File is outside upload directory"
            ));
        }
        
        fs::remove_file(file_path)
    }
    
    /// Get storage configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    /// List files in upload directory
    pub fn list_files(&self) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_files(&self.config.upload_dir, &mut files)?;
        Ok(files)
    }
    
    /// Recursively collect files from directory
    fn collect_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                self.collect_files(&path, files)?;
            }
        }
        Ok(())
    }
}

// Simple random number generation for filenames
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn random<T>() -> T 
    where 
        T: From<u32>
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        T::from(nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_sanitize_filename() {
        let storage = FileStorage::new(StorageConfig::default()).unwrap();
        
        assert_eq!(storage.sanitize_filename("test.txt"), "test.txt");
        assert_eq!(storage.sanitize_filename("../../../etc/passwd"), "______etc_passwd");
        assert_eq!(storage.sanitize_filename("file with spaces.txt"), "file_with_spaces.txt");
        assert_eq!(storage.sanitize_filename("file:with*bad?chars.txt"), "file_with_bad_chars.txt");
    }
    
    #[test]
    fn test_extension_from_content_type() {
        let storage = FileStorage::new(StorageConfig::default()).unwrap();
        
        assert_eq!(storage.extension_from_content_type("image/jpeg"), Some("jpg".to_string()));
        assert_eq!(storage.extension_from_content_type("text/plain"), Some("txt".to_string()));
        assert_eq!(storage.extension_from_content_type("application/unknown"), None);
    }
    
    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.upload_dir, PathBuf::from("./uploads"));
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(config.preserve_filenames);
        assert!(!config.use_date_subdirs);
    }
}
