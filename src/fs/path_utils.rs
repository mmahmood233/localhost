use std::path::{Path, PathBuf};
use std::io;

/// Safely resolve a request path within a document root, preventing directory traversal attacks
pub fn safe_path_join(document_root: &Path, request_path: &str) -> io::Result<PathBuf> {
    // Remove leading slash and normalize the path
    let clean_path = request_path.trim_start_matches('/');
    
    // Split into components and validate each one
    let mut safe_components = Vec::new();
    for component in clean_path.split('/') {
        match component {
            "" | "." => {
                // Skip empty components and current directory references
                continue;
            }
            ".." => {
                // Directory traversal attempt - reject
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Directory traversal not allowed"
                ));
            }
            comp if comp.contains('\0') => {
                // Null bytes in path - reject
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Null bytes in path not allowed"
                ));
            }
            comp => {
                safe_components.push(comp);
            }
        }
    }
    
    // Build the safe path
    let mut result = document_root.to_path_buf();
    for component in safe_components {
        result.push(component);
    }
    
    // Ensure the result is still within the document root
    let canonical_root = document_root.canonicalize()
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "Document root not found"))?;
    
    // For security, we need to check if the resolved path would escape the document root
    // We do this by checking if the canonical document root is a prefix of our result
    if let Ok(canonical_result) = result.canonicalize() {
        if !canonical_result.starts_with(&canonical_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Path escapes document root"
            ));
        }
    }
    
    Ok(result)
}

/// Check if a path represents a directory and should serve an index file
pub fn should_serve_index(path: &Path) -> bool {
    path.is_dir()
}

/// Get the index file path for a directory
pub fn get_index_path(dir_path: &Path, index_file: &str) -> PathBuf {
    dir_path.join(index_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_safe_path_join_normal_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Normal paths should work
        let result = safe_path_join(root, "/index.html").unwrap();
        assert_eq!(result, root.join("index.html"));
        
        let result = safe_path_join(root, "/css/style.css").unwrap();
        assert_eq!(result, root.join("css").join("style.css"));
        
        let result = safe_path_join(root, "js/script.js").unwrap();
        assert_eq!(result, root.join("js").join("script.js"));
    }
    
    #[test]
    fn test_safe_path_join_directory_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Directory traversal attempts should be rejected
        assert!(safe_path_join(root, "/../etc/passwd").is_err());
        assert!(safe_path_join(root, "/css/../../../etc/passwd").is_err());
        assert!(safe_path_join(root, "/./../../etc/passwd").is_err());
    }
    
    #[test]
    fn test_safe_path_join_null_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Null bytes should be rejected
        assert!(safe_path_join(root, "/index.html\0").is_err());
        assert!(safe_path_join(root, "/css/\0style.css").is_err());
    }
    
    #[test]
    fn test_index_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create a directory
        let css_dir = root.join("css");
        fs::create_dir(&css_dir).unwrap();
        
        assert!(should_serve_index(&css_dir));
        assert!(!should_serve_index(&root.join("nonexistent.html")));
        
        let index_path = get_index_path(&css_dir, "index.html");
        assert_eq!(index_path, css_dir.join("index.html"));
    }
}
