use std::fs::{File, Metadata};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::fs::path_utils::{safe_path_join, should_serve_index, get_index_path};
use crate::mime::MimeTypes;
use crate::http::response::HttpResponse;

pub struct StaticFileServer {
    document_root: PathBuf,
    mime_types: MimeTypes,
    index_file: String,
}

impl StaticFileServer {
    pub fn new<P: AsRef<Path>>(document_root: P, index_file: Option<String>) -> io::Result<Self> {
        let root = document_root.as_ref().to_path_buf();
        
        // Ensure document root exists and is a directory
        if !root.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Document root does not exist: {}", root.display())
            ));
        }
        
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Document root is not a directory: {}", root.display())
            ));
        }
        
        Ok(StaticFileServer {
            document_root: root,
            mime_types: MimeTypes::new(),
            index_file: index_file.unwrap_or_else(|| "index.html".to_string()),
        })
    }
    
    /// Serve a static file for the given request path
    pub fn serve_file(&self, request_path: &str) -> io::Result<HttpResponse> {
        // Resolve the safe file path
        let file_path = safe_path_join(&self.document_root, request_path)?;
        
        // Check if we should serve an index file for a directory
        let final_path = if should_serve_index(&file_path) {
            get_index_path(&file_path, &self.index_file)
        } else {
            file_path
        };
        
        // Try to open and read the file
        match self.read_file(&final_path) {
            Ok((content, metadata)) => {
                let mut response = HttpResponse::ok();
                
                // Set content type based on file extension
                let mime_type = self.mime_types.get_mime_type(&final_path);
                response.set_header("Content-Type", mime_type);
                
                // Set content length
                response.set_body(&content);
                
                // Set Last-Modified header
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        response.set_header("Last-Modified", &format!("timestamp-{}", timestamp));
                    }
                }
                
                // Set caching headers for static assets
                if self.is_cacheable_asset(&final_path) {
                    response.set_header("Cache-Control", "public, max-age=3600");
                }
                
                Ok(response)
            }
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        Ok(HttpResponse::not_found())
                    }
                    io::ErrorKind::PermissionDenied => {
                        let mut response = HttpResponse::new(403);
                        response.set_body_string("403 Forbidden");
                        response.set_header("Content-Type", "text/plain");
                        Ok(response)
                    }
                    _ => {
                        eprintln!("Error serving file {}: {}", final_path.display(), e);
                        Ok(HttpResponse::internal_server_error())
                    }
                }
            }
        }
    }
    
    fn read_file(&self, path: &Path) -> io::Result<(Vec<u8>, Metadata)> {
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        
        // Check file size to prevent reading extremely large files
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
        if metadata.len() > MAX_FILE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File too large to serve"
            ));
        }
        
        let mut content = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut content)?;
        
        Ok((content, metadata))
    }
    
    fn is_cacheable_asset(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext_lower = ext_str.to_lowercase();
                return matches!(ext_lower.as_str(), 
                    "css" | "js" | "png" | "jpg" | "jpeg" | "gif" | "svg" | 
                    "ico" | "woff" | "woff2" | "ttf" | "otf"
                );
            }
        }
        false
    }
    
    pub fn document_root(&self) -> &Path {
        &self.document_root
    }
    
    pub fn index_file(&self) -> &str {
        &self.index_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_static_file_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Should succeed with valid directory
        let server = StaticFileServer::new(root, None).unwrap();
        assert_eq!(server.document_root(), root);
        assert_eq!(server.index_file(), "index.html");
        
        // Should succeed with custom index file
        let server = StaticFileServer::new(root, Some("home.html".to_string())).unwrap();
        assert_eq!(server.index_file(), "home.html");
        
        // Should fail with non-existent directory
        let bad_path = root.join("nonexistent");
        assert!(StaticFileServer::new(&bad_path, None).is_err());
    }
    
    #[test]
    fn test_serve_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create a test file
        let test_content = "Hello, World!";
        fs::write(root.join("test.txt"), test_content).unwrap();
        
        let server = StaticFileServer::new(root, None).unwrap();
        let response = server.serve_file("/test.txt").unwrap();
        
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, test_content.as_bytes());
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/plain".to_string()));
    }
    
    #[test]
    fn test_serve_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        let server = StaticFileServer::new(root, None).unwrap();
        let response = server.serve_file("/nonexistent.txt").unwrap();
        
        assert_eq!(response.status_code, 404);
    }
    
    #[test]
    fn test_serve_directory_with_index() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create index file
        let index_content = "<html><body>Index Page</body></html>";
        fs::write(root.join("index.html"), index_content).unwrap();
        
        let server = StaticFileServer::new(root, None).unwrap();
        let response = server.serve_file("/").unwrap();
        
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, index_content.as_bytes());
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/html".to_string()));
    }
    
    #[test]
    fn test_directory_traversal_protection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        let server = StaticFileServer::new(root, None).unwrap();
        
        // These should all fail safely
        assert!(server.serve_file("/../etc/passwd").is_err() || 
                server.serve_file("/../etc/passwd").unwrap().status_code == 404);
        assert!(server.serve_file("/../../etc/passwd").is_err() || 
                server.serve_file("/../../etc/passwd").unwrap().status_code == 404);
    }
}
