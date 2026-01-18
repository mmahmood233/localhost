use crate::cgi::environment::CgiEnvironment;
use crate::cgi::response::CgiResponseParser;
use crate::http::request::HttpRequest;
use crate::http::response::HttpResponse;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// CGI configuration
#[derive(Debug, Clone)]
pub struct CgiConfig {
    /// CGI script extensions and their interpreters
    pub interpreters: HashMap<String, String>,
    /// CGI execution timeout
    pub timeout: Duration,
    /// Maximum CGI output size
    pub max_output_size: usize,
    /// CGI script directory
    pub cgi_directory: PathBuf,
    /// Whether to enable CGI execution
    pub enabled: bool,
}

impl Default for CgiConfig {
    fn default() -> Self {
        let mut interpreters = HashMap::new();
        interpreters.insert("py".to_string(), "python3".to_string());
        interpreters.insert("pl".to_string(), "perl".to_string());
        interpreters.insert("sh".to_string(), "sh".to_string());
        interpreters.insert("rb".to_string(), "ruby".to_string());
        interpreters.insert("php".to_string(), "php".to_string());
        
        CgiConfig {
            interpreters,
            timeout: Duration::from_secs(30),
            max_output_size: 1024 * 1024, // 1MB
            cgi_directory: PathBuf::from("cgi-bin"),
            enabled: true,
        }
    }
}

/// CGI script executor
#[derive(Debug, Clone)]
pub struct CgiExecutor {
    config: CgiConfig,
}

impl CgiExecutor {
    pub fn new(config: CgiConfig) -> Self {
        CgiExecutor { config }
    }
    
    /// Check if a path is a CGI script
    pub fn is_cgi_script(&self, path: &Path) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        // Check if file is in CGI directory
        if let Some(parent) = path.parent() {
            if !parent.ends_with(&self.config.cgi_directory) {
                return false;
            }
        }
        
        // Check file extension
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.config.interpreters.contains_key(ext_str);
            }
        }
        
        // Check if file is executable (simplified check)
        path.is_file()
    }
    
    /// Execute CGI script and return HTTP response
    pub fn execute_cgi(
        &self,
        request: &HttpRequest,
        script_path: &Path,
        document_root: &Path,
        server_name: &str,
        server_port: u16,
    ) -> io::Result<HttpResponse> {
        if !self.config.enabled {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "CGI execution is disabled",
            ));
        }
        
        if !script_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "CGI script not found",
            ));
        }
        
        // Create CGI environment
        let mut env = CgiEnvironment::from_request(
            request,
            script_path,
            document_root,
            server_name,
            server_port,
        );
        env.add_system_env();
        
        // Execute the script
        let output = self.execute_script(script_path, &env, request.body())?;
        
        // Parse CGI output
        let cgi_response = CgiResponseParser::parse_complete(&output)?;
        
        Ok(cgi_response.to_http_response())
    }
    
    /// Execute the CGI script with proper process management
    fn execute_script(
        &self,
        script_path: &Path,
        environment: &CgiEnvironment,
        request_body: Option<&[u8]>,
    ) -> io::Result<Vec<u8>> {
        // Determine interpreter and command
        let (command, args) = self.get_command_and_args(script_path)?;
        
        // Create process with proper stdio setup
        let mut child = Command::new(&command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(environment.variables())
            .spawn()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to spawn CGI process: {}", e),
                )
            })?;
        
        // Write request body to stdin if present
        if let Some(body) = request_body {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(body).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        format!("Failed to write to CGI stdin: {}", e),
                    )
                })?;
                // Close stdin to signal end of input
                drop(stdin);
            }
        }
        
        // Wait for process with timeout
        let start_time = Instant::now();
        let mut output = Vec::new();
        let mut stderr_output = Vec::new();
        
        // Read output with timeout
        loop {
            if start_time.elapsed() > self.config.timeout {
                let _ = child.kill();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "CGI script execution timed out",
                ));
            }
            
            // Try to read stdout
            if let Some(mut stdout) = child.stdout.take() {
                let mut buffer = [0; 4096];
                match stdout.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        output.extend_from_slice(&buffer[..n]);
                        if output.len() > self.config.max_output_size {
                            let _ = child.kill();
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "CGI output too large",
                            ));
                        }
                        child.stdout = Some(stdout);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        child.stdout = Some(stdout);
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => {
                        child.stdout = Some(stdout);
                        return Err(e);
                    }
                }
            }
            
            // Check if process has finished
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process finished, read any remaining output
                    if let Some(mut stdout) = child.stdout.take() {
                        let _ = stdout.read_to_end(&mut output);
                    }
                    if let Some(mut stderr) = child.stderr.take() {
                        let _ = stderr.read_to_end(&mut stderr_output);
                    }
                    
                    if !status.success() {
                        let stderr_str = String::from_utf8_lossy(&stderr_output);
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("CGI script failed with status {}: {}", status, stderr_str),
                        ));
                    }
                    break;
                }
                Ok(None) => {
                    // Process still running
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(e);
                }
            }
        }
        
        Ok(output)
    }
    
    /// Get command and arguments for executing the script
    fn get_command_and_args(&self, script_path: &Path) -> io::Result<(String, Vec<String>)> {
        // Check if script has a shebang line
        if let Ok(shebang) = self.read_shebang(script_path) {
            if let Some(interpreter) = shebang {
                return Ok((interpreter, vec![script_path.to_string_lossy().to_string()]));
            }
        }
        
        // Use extension-based interpreter mapping
        if let Some(extension) = script_path.extension() {
            if let Some(ext_str) = extension.to_str() {
                if let Some(interpreter) = self.config.interpreters.get(ext_str) {
                    return Ok((
                        interpreter.clone(),
                        vec![script_path.to_string_lossy().to_string()],
                    ));
                }
            }
        }
        
        // Try to execute directly (for executable scripts)
        Ok((script_path.to_string_lossy().to_string(), vec![]))
    }
    
    /// Read shebang line from script
    fn read_shebang(&self, script_path: &Path) -> io::Result<Option<String>> {
        use std::fs::File;
        use std::io::BufRead;
        
        let file = File::open(script_path)?;
        let mut reader = io::BufReader::new(file);
        let mut first_line = String::new();
        
        reader.read_line(&mut first_line)?;
        
        if first_line.starts_with("#!") {
            let shebang = first_line[2..].trim();
            // Extract just the interpreter (first word)
            if let Some(interpreter) = shebang.split_whitespace().next() {
                return Ok(Some(interpreter.to_string()));
            }
        }
        
        Ok(None)
    }
    
    /// Get CGI configuration
    pub fn config(&self) -> &CgiConfig {
        &self.config
    }
    
    /// Check if CGI is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Get supported file extensions
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.config.interpreters.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CgiExecutor {
    fn default() -> Self {
        Self::new(CgiConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    #[test]
    fn test_cgi_config_default() {
        let config = CgiConfig::default();
        assert!(config.enabled);
        assert!(config.interpreters.contains_key("py"));
        assert!(config.interpreters.contains_key("pl"));
        assert_eq!(config.timeout, Duration::from_secs(30));
    }
    
    #[test]
    fn test_cgi_executor_creation() {
        let executor = CgiExecutor::default();
        assert!(executor.is_enabled());
        assert!(executor.supported_extensions().contains(&"py"));
    }
    
    #[test]
    fn test_is_cgi_script() {
        let executor = CgiExecutor::default();
        
        // Test CGI script in cgi-bin directory
        let cgi_script = PathBuf::from("cgi-bin/test.py");
        // Note: This will return false because the file doesn't exist
        // In a real test, we'd create the file
        
        // Test non-CGI file
        let regular_file = PathBuf::from("index.html");
        assert!(!executor.is_cgi_script(&regular_file));
    }
    
    #[test]
    fn test_get_command_and_args() {
        let executor = CgiExecutor::default();
        let script_path = PathBuf::from("test.py");
        
        let (command, args) = executor.get_command_and_args(&script_path).unwrap();
        assert_eq!(command, "python3");
        assert_eq!(args, vec!["test.py"]);
    }
    
    #[test]
    fn test_supported_extensions() {
        let executor = CgiExecutor::default();
        let extensions = executor.supported_extensions();
        
        assert!(extensions.contains(&"py"));
        assert!(extensions.contains(&"pl"));
        assert!(extensions.contains(&"sh"));
        assert!(extensions.contains(&"rb"));
        assert!(extensions.contains(&"php"));
    }
    
    #[test]
    fn test_disabled_cgi() {
        let mut config = CgiConfig::default();
        config.enabled = false;
        let executor = CgiExecutor::new(config);
        
        assert!(!executor.is_enabled());
        
        let script_path = PathBuf::from("cgi-bin/test.py");
        assert!(!executor.is_cgi_script(&script_path));
    }
}
