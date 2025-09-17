use std::collections::HashMap;
use std::path::Path;

pub struct MimeTypes {
    types: HashMap<String, &'static str>,
}

impl MimeTypes {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        
        // Text types
        types.insert("html".to_string(), "text/html");
        types.insert("htm".to_string(), "text/html");
        types.insert("css".to_string(), "text/css");
        types.insert("js".to_string(), "application/javascript");
        types.insert("json".to_string(), "application/json");
        types.insert("xml".to_string(), "application/xml");
        types.insert("txt".to_string(), "text/plain");
        types.insert("md".to_string(), "text/markdown");
        
        // Image types
        types.insert("png".to_string(), "image/png");
        types.insert("jpg".to_string(), "image/jpeg");
        types.insert("jpeg".to_string(), "image/jpeg");
        types.insert("gif".to_string(), "image/gif");
        types.insert("svg".to_string(), "image/svg+xml");
        types.insert("ico".to_string(), "image/x-icon");
        types.insert("webp".to_string(), "image/webp");
        
        // Font types
        types.insert("woff".to_string(), "font/woff");
        types.insert("woff2".to_string(), "font/woff2");
        types.insert("ttf".to_string(), "font/ttf");
        types.insert("otf".to_string(), "font/otf");
        
        // Application types
        types.insert("pdf".to_string(), "application/pdf");
        types.insert("zip".to_string(), "application/zip");
        types.insert("tar".to_string(), "application/x-tar");
        types.insert("gz".to_string(), "application/gzip");
        
        // Video types
        types.insert("mp4".to_string(), "video/mp4");
        types.insert("webm".to_string(), "video/webm");
        types.insert("ogg".to_string(), "video/ogg");
        
        // Audio types
        types.insert("mp3".to_string(), "audio/mpeg");
        types.insert("wav".to_string(), "audio/wav");
        types.insert("flac".to_string(), "audio/flac");
        
        MimeTypes { types }
    }
    
    pub fn get_mime_type(&self, path: &Path) -> &'static str {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext_lower = ext_str.to_lowercase();
                return self.types.get(&ext_lower).copied().unwrap_or("application/octet-stream");
            }
        }
        "application/octet-stream"
    }
    
    pub fn is_text_type(&self, mime_type: &str) -> bool {
        mime_type.starts_with("text/") || 
        mime_type == "application/javascript" ||
        mime_type == "application/json" ||
        mime_type == "application/xml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_mime_types() {
        let mime_types = MimeTypes::new();
        
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("index.html")), "text/html");
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("style.css")), "text/css");
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("script.js")), "application/javascript");
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("image.png")), "image/png");
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("unknown.xyz")), "application/octet-stream");
        assert_eq!(mime_types.get_mime_type(&PathBuf::from("noextension")), "application/octet-stream");
    }
    
    #[test]
    fn test_text_type_detection() {
        let mime_types = MimeTypes::new();
        
        assert!(mime_types.is_text_type("text/html"));
        assert!(mime_types.is_text_type("text/plain"));
        assert!(mime_types.is_text_type("application/javascript"));
        assert!(mime_types.is_text_type("application/json"));
        assert!(!mime_types.is_text_type("image/png"));
        assert!(!mime_types.is_text_type("application/octet-stream"));
    }
}
