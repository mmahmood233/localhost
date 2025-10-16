//! Error handling and HTTP error responses

use crate::http::response::HttpResponse;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Custom error page configuration
#[derive(Debug, Clone)]
pub struct ErrorPageConfig {
    /// Custom error page files mapped by status code
    pub custom_pages: HashMap<u16, PathBuf>,
    /// Default error page directory
    pub error_dir: Option<PathBuf>,
    /// Whether to show detailed error information
    pub show_details: bool,
    /// Server name to include in error pages
    pub server_name: String,
}

impl Default for ErrorPageConfig {
    fn default() -> Self {
        ErrorPageConfig {
            custom_pages: HashMap::new(),
            error_dir: None,
            show_details: false,
            server_name: "Localhost HTTP Server".to_string(),
        }
    }
}

/// Error page generator
#[derive(Debug)]
pub struct ErrorPageGenerator {
    config: ErrorPageConfig,
    /// Cache for loaded custom error pages
    page_cache: HashMap<u16, String>,
}

impl ErrorPageGenerator {
    /// Create new error page generator
    pub fn new(config: ErrorPageConfig) -> Self {
        ErrorPageGenerator {
            config,
            page_cache: HashMap::new(),
        }
    }
    
    /// Generate error response for given status code
    pub fn generate_error_response(&mut self, status: u16, message: Option<&str>, details: Option<&str>) -> HttpResponse {
        let body = self.generate_error_page(status, message, details);
        
        let mut response = HttpResponse::new(status);
        response.set_header("Content-Type", "text/html; charset=utf-8");
        response.set_header("Content-Length", &body.len().to_string());
        response.set_header("Cache-Control", "no-cache, no-store, must-revalidate");
        response.set_header("Pragma", "no-cache");
        response.set_header("Expires", "0");
        response.set_body(body.as_bytes());
        
        response
    }
    
    /// Generate error page HTML
    pub fn generate_error_page(&mut self, status: u16, message: Option<&str>, details: Option<&str>) -> String {
        // Try to load custom error page first
        if let Some(custom_page) = self.load_custom_error_page(status) {
            return self.substitute_variables(&custom_page, status, message, details);
        }
        
        // Generate default error page
        self.generate_default_error_page(status, message, details)
    }
    
    /// Load custom error page from file
    fn load_custom_error_page(&mut self, status: u16) -> Option<String> {
        // Check cache first
        if let Some(cached_page) = self.page_cache.get(&status) {
            return Some(cached_page.clone());
        }
        
        // Try to load from custom pages
        if let Some(page_path) = self.config.custom_pages.get(&status) {
            if let Ok(content) = fs::read_to_string(page_path) {
                self.page_cache.insert(status, content.clone());
                return Some(content);
            }
        }
        
        // Try to load from error directory
        if let Some(ref error_dir) = self.config.error_dir {
            let page_path = error_dir.join(format!("{}.html", status));
            if page_path.exists() {
                if let Ok(content) = fs::read_to_string(&page_path) {
                    self.page_cache.insert(status, content.clone());
                    return Some(content);
                }
            }
        }
        
        None
    }
    
    /// Substitute variables in custom error page
    fn substitute_variables(&self, template: &str, status: u16, message: Option<&str>, details: Option<&str>) -> String {
        let status_text = self.get_status_text(status);
        let message = message.unwrap_or(&status_text);
        let details = if self.config.show_details {
            details.unwrap_or("")
        } else {
            ""
        };
        
        template
            .replace("{{STATUS}}", &status.to_string())
            .replace("{{STATUS_TEXT}}", &status_text)
            .replace("{{MESSAGE}}", message)
            .replace("{{DETAILS}}", details)
            .replace("{{SERVER_NAME}}", &self.config.server_name)
            .replace("{{TIMESTAMP}}", &std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string())
    }
    
    /// Generate default error page HTML
    fn generate_default_error_page(&self, status: u16, message: Option<&str>, details: Option<&str>) -> String {
        let status_text = self.get_status_text(status);
        let message = message.unwrap_or(&status_text);
        
        let mut html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - {}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .error-container {{
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            padding: 3rem;
            text-align: center;
            max-width: 500px;
            margin: 2rem;
        }}
        .error-code {{
            font-size: 4rem;
            font-weight: 700;
            color: #e74c3c;
            margin: 0;
            line-height: 1;
        }}
        .error-message {{
            font-size: 1.5rem;
            color: #2c3e50;
            margin: 1rem 0;
            font-weight: 500;
        }}
        .error-description {{
            color: #7f8c8d;
            font-size: 1rem;
            line-height: 1.6;
            margin: 1.5rem 0;
        }}
        .error-details {{
            background: #f8f9fa;
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 1rem;
            margin: 1.5rem 0;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 0.875rem;
            color: #495057;
            text-align: left;
            white-space: pre-wrap;
        }}
        .server-info {{
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid #e9ecef;
            color: #6c757d;
            font-size: 0.875rem;
        }}
        .back-link {{
            display: inline-block;
            margin-top: 1.5rem;
            padding: 0.75rem 1.5rem;
            background: #007bff;
            color: white;
            text-decoration: none;
            border-radius: 6px;
            transition: background-color 0.2s;
        }}
        .back-link:hover {{
            background: #0056b3;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <h1 class="error-code">{}</h1>
        <h2 class="error-message">{}</h2>
        <p class="error-description">{}</p>"#,
            status, status_text, status, message, self.get_status_description(status)
        );
        
        // Add details if enabled and available
        if self.config.show_details {
            if let Some(details) = details {
                if !details.is_empty() {
                    html.push_str(&format!(
                        r#"        <div class="error-details">{}</div>"#,
                        html_escape(details)
                    ));
                }
            }
        }
        
        // Add navigation and server info
        html.push_str(&format!(
            r#"        <a href="/" class="back-link">← Go Home</a>
        <div class="server-info">
            {} • {}
        </div>
    </div>
</body>
</html>"#,
            self.config.server_name,
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        ));
        
        html
    }
    
    /// Get HTTP status text
    fn get_status_text(&self, status: u16) -> String {
        match status {
            400 => "Bad Request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not Found".to_string(),
            405 => "Method Not Allowed".to_string(),
            408 => "Request Timeout".to_string(),
            413 => "Payload Too Large".to_string(),
            414 => "URI Too Long".to_string(),
            429 => "Too Many Requests".to_string(),
            500 => "Internal Server Error".to_string(),
            501 => "Not Implemented".to_string(),
            502 => "Bad Gateway".to_string(),
            503 => "Service Unavailable".to_string(),
            504 => "Gateway Timeout".to_string(),
            _ => format!("HTTP Error {}", status),
        }
    }
    
    /// Get detailed status description
    fn get_status_description(&self, status: u16) -> &'static str {
        match status {
            400 => "The server cannot process the request due to invalid syntax.",
            401 => "Authentication is required to access this resource.",
            403 => "You don't have permission to access this resource.",
            404 => "The requested resource could not be found on this server.",
            405 => "The request method is not allowed for this resource.",
            408 => "The server timed out waiting for the request.",
            413 => "The request payload is too large for the server to process.",
            414 => "The request URI is too long for the server to process.",
            429 => "Too many requests have been sent in a given amount of time.",
            500 => "The server encountered an unexpected condition.",
            501 => "The server does not support the functionality required.",
            502 => "The server received an invalid response from an upstream server.",
            503 => "The server is temporarily unavailable.",
            504 => "The server timed out waiting for an upstream server.",
            _ => "An error occurred while processing your request.",
        }
    }
    
    /// Clear error page cache
    pub fn clear_cache(&mut self) {
        self.page_cache.clear();
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: ErrorPageConfig) {
        self.config = config;
        self.clear_cache();
    }
    
    /// Get current configuration
    pub fn config(&self) -> &ErrorPageConfig {
        &self.config
    }
}

/// Directory listing generator
#[derive(Debug)]
pub struct DirectoryListing {
    /// Whether directory listing is enabled
    pub enabled: bool,
    /// Show hidden files (starting with .)
    pub show_hidden: bool,
    /// Show file sizes
    pub show_sizes: bool,
    /// Show modification times
    pub show_times: bool,
    /// Custom CSS for directory listing
    pub custom_css: Option<String>,
}

impl Default for DirectoryListing {
    fn default() -> Self {
        DirectoryListing {
            enabled: true,
            show_hidden: false,
            show_sizes: true,
            show_times: true,
            custom_css: None,
        }
    }
}

impl DirectoryListing {
    /// Generate directory listing HTML
    pub fn generate_listing(&self, dir_path: &Path, request_path: &str) -> io::Result<String> {
        if !self.enabled {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Directory listing disabled"));
        }
        
        let entries = fs::read_dir(dir_path)?;
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files if not enabled
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }
            
            let file_info = FileInfo {
                name: file_name,
                path: entry.path(),
                size: metadata.len(),
                modified: metadata.modified().ok(),
                is_dir: metadata.is_dir(),
            };
            
            if file_info.is_dir {
                dirs.push(file_info);
            } else {
                files.push(file_info);
            }
        }
        
        // Sort directories and files separately
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        
        self.generate_html(request_path, &dirs, &files)
    }
    
    /// Generate HTML for directory listing
    fn generate_html(&self, request_path: &str, dirs: &[FileInfo], files: &[FileInfo]) -> io::Result<String> {
        let title = format!("Index of {}", request_path);
        
        let mut html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 2rem;
            background: #f8f9fa;
            color: #212529;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        .header {{
            background: #007bff;
            color: white;
            padding: 1.5rem 2rem;
            border-bottom: 1px solid #0056b3;
        }}
        .header h1 {{
            margin: 0;
            font-size: 1.5rem;
            font-weight: 500;
        }}
        .breadcrumb {{
            margin: 0.5rem 0 0 0;
            font-size: 0.875rem;
            opacity: 0.9;
        }}
        .breadcrumb a {{
            color: #cce7ff;
            text-decoration: none;
        }}
        .breadcrumb a:hover {{
            text-decoration: underline;
        }}
        .listing {{
            padding: 0;
        }}
        .listing-table {{
            width: 100%;
            border-collapse: collapse;
        }}
        .listing-table th {{
            background: #f8f9fa;
            padding: 1rem 2rem;
            text-align: left;
            font-weight: 600;
            color: #495057;
            border-bottom: 1px solid #dee2e6;
        }}
        .listing-table td {{
            padding: 0.75rem 2rem;
            border-bottom: 1px solid #f1f3f4;
        }}
        .listing-table tr:hover {{
            background: #f8f9fa;
        }}
        .file-icon {{
            width: 20px;
            height: 20px;
            margin-right: 0.5rem;
            vertical-align: middle;
        }}
        .file-name {{
            display: flex;
            align-items: center;
        }}
        .file-name a {{
            color: #007bff;
            text-decoration: none;
        }}
        .file-name a:hover {{
            text-decoration: underline;
        }}
        .dir-icon {{
            color: #ffc107;
        }}
        .file-icon-default {{
            color: #6c757d;
        }}
        .file-size {{
            color: #6c757d;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 0.875rem;
        }}
        .file-time {{
            color: #6c757d;
            font-size: 0.875rem;
        }}
        .parent-dir {{
            font-weight: 600;
        }}
        .footer {{
            padding: 1rem 2rem;
            background: #f8f9fa;
            color: #6c757d;
            font-size: 0.875rem;
            text-align: center;
            border-top: 1px solid #dee2e6;
        }}
    </style>"#,
            title
        );
        
        // Add custom CSS if provided
        if let Some(ref custom_css) = self.custom_css {
            html.push_str(&format!("    <style>{}</style>", custom_css));
        }
        
        html.push_str(&format!(
            r#"</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Index of {}</h1>
            <div class="breadcrumb">{}</div>
        </div>
        <div class="listing">
            <table class="listing-table">
                <thead>
                    <tr>
                        <th>Name</th>"#,
            request_path,
            self.generate_breadcrumb(request_path)
        ));
        
        if self.show_sizes {
            html.push_str("                        <th>Size</th>");
        }
        
        if self.show_times {
            html.push_str("                        <th>Modified</th>");
        }
        
        html.push_str(
            r#"                    </tr>
                </thead>
                <tbody>"#
        );
        
        // Add parent directory link if not at root
        if request_path != "/" {
            let parent_path = if request_path.ends_with('/') {
                format!("{}../", request_path)
            } else {
                format!("{}/", request_path.rsplitn(2, '/').nth(1).unwrap_or(""))
            };
            
            html.push_str(&format!(
                r#"                    <tr>
                        <td class="file-name parent-dir">
                            <span class="file-icon dir-icon">📁</span>
                            <a href="{}">../</a>
                        </td>"#,
                parent_path
            ));
            
            if self.show_sizes {
                html.push_str("                        <td>-</td>");
            }
            if self.show_times {
                html.push_str("                        <td>-</td>");
            }
            
            html.push_str("                    </tr>");
        }
        
        // Add directories
        for dir in dirs {
            html.push_str(&self.generate_file_row(dir, request_path));
        }
        
        // Add files
        for file in files {
            html.push_str(&self.generate_file_row(file, request_path));
        }
        
        html.push_str(&format!(
            r#"                </tbody>
            </table>
        </div>
        <div class="footer">
            {} items • Generated by Localhost HTTP Server
        </div>
    </div>
</body>
</html>"#,
            dirs.len() + files.len()
        ));
        
        Ok(html)
    }
    
    /// Generate breadcrumb navigation
    fn generate_breadcrumb(&self, path: &str) -> String {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut breadcrumb = String::from(r#"<a href="/">Home</a>"#);
        
        let mut current_path = String::new();
        for part in parts {
            current_path.push('/');
            current_path.push_str(part);
            breadcrumb.push_str(&format!(r#" / <a href="{}">{}</a>"#, current_path, part));
        }
        
        breadcrumb
    }
    
    /// Generate table row for file/directory
    fn generate_file_row(&self, file: &FileInfo, request_path: &str) -> String {
        let href = if request_path.ends_with('/') {
            format!("{}{}", request_path, file.name)
        } else {
            format!("{}/{}", request_path, file.name)
        };
        
        let icon = if file.is_dir {
            "📁"
        } else {
            self.get_file_icon(&file.name)
        };
        
        let icon_class = if file.is_dir { "dir-icon" } else { "file-icon-default" };
        
        let mut row = format!(
            r#"                    <tr>
                        <td class="file-name">
                            <span class="file-icon {}">{}</span>
                            <a href="{}">{}{}</a>
                        </td>"#,
            icon_class, icon, href, file.name, if file.is_dir { "/" } else { "" }
        );
        
        if self.show_sizes {
            let size_str = if file.is_dir {
                "-".to_string()
            } else {
                format_file_size(file.size)
            };
            row.push_str(&format!(r#"                        <td class="file-size">{}</td>"#, size_str));
        }
        
        if self.show_times {
            let time_str = if let Some(modified) = file.modified {
                format_time(modified)
            } else {
                "-".to_string()
            };
            row.push_str(&format!(r#"                        <td class="file-time">{}</td>"#, time_str));
        }
        
        row.push_str("                    </tr>");
        row
    }
    
    /// Get file icon based on extension
    fn get_file_icon(&self, filename: &str) -> &'static str {
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();
        match extension.as_str() {
            "html" | "htm" => "🌐",
            "css" => "🎨",
            "js" => "📜",
            "json" => "📋",
            "xml" => "📄",
            "txt" | "md" => "📝",
            "pdf" => "📕",
            "doc" | "docx" => "📘",
            "xls" | "xlsx" => "📗",
            "ppt" | "pptx" => "📙",
            "zip" | "tar" | "gz" | "rar" => "📦",
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "🖼️",
            "mp3" | "wav" | "ogg" | "flac" => "🎵",
            "mp4" | "avi" | "mkv" | "mov" => "🎬",
            "py" => "🐍",
            "rs" => "🦀",
            "go" => "🐹",
            "java" => "☕",
            "c" | "cpp" | "h" => "⚙️",
            _ => "📄",
        }
    }
}

/// File information for directory listing
#[derive(Debug, Clone)]
struct FileInfo {
    name: String,
    path: PathBuf,
    size: u64,
    modified: Option<std::time::SystemTime>,
    is_dir: bool,
}

/// HTML escape utility
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Format file size in human readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format system time for display
fn format_time(time: std::time::SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            let timestamp = duration.as_secs();
            // Simple timestamp formatting without external dependencies
            format!("{}", timestamp)
        }
        Err(_) => "-".to_string(),
    }
}
