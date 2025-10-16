use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

/// Type of multipart field
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Text field with string value
    Text(String),
    /// File field with file data
    File {
        filename: Option<String>,
        content_type: Option<String>,
        data: Vec<u8>,
    },
}

/// A single field in a multipart form
#[derive(Debug, Clone)]
pub struct MultipartField {
    pub name: String,
    pub field_type: FieldType,
}

/// Parser for multipart/form-data content
pub struct MultipartParser {
    boundary: String,
    max_file_size: usize,
    max_total_size: usize,
}

impl MultipartParser {
    pub fn new(boundary: String, max_file_size: usize, max_total_size: usize) -> Self {
        MultipartParser {
            boundary,
            max_file_size,
            max_total_size,
        }
    }
    
    /// Parse multipart form data from request body
    pub fn parse(&self, data: &[u8]) -> io::Result<Vec<MultipartField>> {
        if data.len() > self.max_total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Request body too large"
            ));
        }
        
        let boundary_bytes = format!("--{}", self.boundary).into_bytes();
        let end_boundary_bytes = format!("--{}--", self.boundary).into_bytes();
        
        let mut fields = Vec::new();
        let mut pos = 0;
        
        // Skip initial boundary
        if let Some(boundary_pos) = self.find_boundary(data, &boundary_bytes, pos) {
            pos = boundary_pos + boundary_bytes.len();
            if pos < data.len() && data[pos] == b'\r' {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'\n' {
                pos += 1;
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid multipart data: missing initial boundary"
            ));
        }
        
        // Parse each part
        while pos < data.len() {
            // Find next boundary or end boundary
            let next_boundary_pos = self.find_boundary(data, &boundary_bytes, pos);
            let end_boundary_pos = self.find_boundary(data, &end_boundary_bytes, pos);
            
            let part_end = match (next_boundary_pos, end_boundary_pos) {
                (Some(next), Some(end)) => std::cmp::min(next, end),
                (Some(next), None) => next,
                (None, Some(end)) => end,
                (None, None) => break,
            };
            
            // Parse this part
            if let Ok(field) = self.parse_part(&data[pos..part_end]) {
                fields.push(field);
            }
            
            // Move to next part
            pos = part_end + boundary_bytes.len();
            if pos < data.len() && data[pos] == b'\r' {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'\n' {
                pos += 1;
            }
            
            // Check if we hit the end boundary
            if end_boundary_pos.is_some() && part_end == end_boundary_pos.unwrap() {
                break;
            }
        }
        
        Ok(fields)
    }
    
    /// Find boundary in data starting from position
    fn find_boundary(&self, data: &[u8], boundary: &[u8], start: usize) -> Option<usize> {
        if start >= data.len() {
            return None;
        }
        
        for i in start..=(data.len().saturating_sub(boundary.len())) {
            if data[i..i + boundary.len()] == *boundary {
                return Some(i);
            }
        }
        None
    }
    
    /// Parse a single multipart part
    fn parse_part(&self, data: &[u8]) -> io::Result<MultipartField> {
        // Find the end of headers (double CRLF)
        let header_end = self.find_header_end(data)?;
        let headers_data = &data[..header_end];
        let body_data = &data[header_end + 4..]; // Skip \r\n\r\n
        
        // Parse headers
        let headers = self.parse_headers(headers_data)?;
        
        // Extract Content-Disposition header
        let content_disposition = headers.get("content-disposition")
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing Content-Disposition header"
            ))?;
        
        let (name, filename) = self.parse_content_disposition(content_disposition)?;
        let content_type = headers.get("content-type").cloned();
        
        // Create field based on whether it's a file or text
        let field_type = if filename.is_some() {
            // File field
            if body_data.len() > self.max_file_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "File too large"
                ));
            }
            
            FieldType::File {
                filename,
                content_type,
                data: body_data.to_vec(),
            }
        } else {
            // Text field
            let text = String::from_utf8(body_data.to_vec())
                .map_err(|_| io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in text field"
                ))?;
            FieldType::Text(text)
        };
        
        Ok(MultipartField {
            name,
            field_type,
        })
    }
    
    /// Find the end of headers section
    fn find_header_end(&self, data: &[u8]) -> io::Result<usize> {
        for i in 0..data.len().saturating_sub(3) {
            if &data[i..i + 4] == b"\r\n\r\n" {
                return Ok(i);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid multipart part: missing header end"
        ))
    }
    
    /// Parse headers from header section
    fn parse_headers(&self, data: &[u8]) -> io::Result<HashMap<String, String>> {
        let headers_str = String::from_utf8(data.to_vec())
            .map_err(|_| io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 in headers"
            ))?;
        
        let mut headers = HashMap::new();
        
        for line in headers_str.lines() {
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(name, value);
            }
        }
        
        Ok(headers)
    }
    
    /// Parse Content-Disposition header to extract name and filename
    fn parse_content_disposition(&self, header: &str) -> io::Result<(String, Option<String>)> {
        let mut name = None;
        let mut filename = None;
        
        // Split by semicolon and parse each part
        for part in header.split(';') {
            let part = part.trim();
            
            if part.starts_with("name=") {
                name = Some(self.parse_quoted_value(&part[5..])?);
            } else if part.starts_with("filename=") {
                filename = Some(self.parse_quoted_value(&part[9..])?);
            }
        }
        
        let name = name.ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing name in Content-Disposition"
        ))?;
        
        Ok((name, filename))
    }
    
    /// Parse a quoted value from header
    fn parse_quoted_value(&self, value: &str) -> io::Result<String> {
        let value = value.trim();
        
        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            Ok(value[1..value.len() - 1].to_string())
        } else {
            Ok(value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multipart_parser_creation() {
        let parser = MultipartParser::new("boundary123".to_string(), 1024 * 1024, 10 * 1024 * 1024);
        assert_eq!(parser.boundary, "boundary123");
        assert_eq!(parser.max_file_size, 1024 * 1024);
        assert_eq!(parser.max_total_size, 10 * 1024 * 1024);
    }
    
    #[test]
    fn test_parse_quoted_value() {
        let parser = MultipartParser::new("test".to_string(), 1024, 1024);
        
        assert_eq!(parser.parse_quoted_value("\"test\"").unwrap(), "test");
        assert_eq!(parser.parse_quoted_value("test").unwrap(), "test");
        assert_eq!(parser.parse_quoted_value("\"file name.txt\"").unwrap(), "file name.txt");
    }
    
    #[test]
    fn test_find_boundary() {
        let parser = MultipartParser::new("boundary".to_string(), 1024, 1024);
        let data = b"some data --boundary more data";
        let boundary = b"--boundary";
        
        assert_eq!(parser.find_boundary(data, boundary, 0), Some(10));
        assert_eq!(parser.find_boundary(data, boundary, 15), None);
    }
}
