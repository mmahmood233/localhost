use std::collections::HashMap;
use std::io;

/// A single form field
#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub value: String,
}

/// Parsed form data from application/x-www-form-urlencoded
#[derive(Debug, Clone)]
pub struct FormData {
    fields: HashMap<String, Vec<String>>,
}

impl FormData {
    pub fn new() -> Self {
        FormData {
            fields: HashMap::new(),
        }
    }
    
    /// Parse URL-encoded form data
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let data_str = String::from_utf8(data.to_vec())
            .map_err(|_| io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 in form data"
            ))?;
        
        let mut form_data = FormData::new();
        
        // Split by & to get key=value pairs
        for pair in data_str.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = &pair[..eq_pos];
                let value = &pair[eq_pos + 1..];
                
                // URL decode key and value
                let decoded_key = Self::url_decode(key)?;
                let decoded_value = Self::url_decode(value)?;
                
                form_data.add_field(decoded_key, decoded_value);
            } else if !pair.is_empty() {
                // Key without value
                let decoded_key = Self::url_decode(pair)?;
                form_data.add_field(decoded_key, String::new());
            }
        }
        
        Ok(form_data)
    }
    
    /// Add a field to the form data
    pub fn add_field(&mut self, name: String, value: String) {
        self.fields.entry(name).or_insert_with(Vec::new).push(value);
    }
    
    /// Get the first value for a field
    pub fn get_field(&self, name: &str) -> Option<&str> {
        self.fields.get(name)?.first().map(|s| s.as_str())
    }
    
    /// Get all values for a field
    pub fn get_field_values(&self, name: &str) -> Option<&Vec<String>> {
        self.fields.get(name)
    }
    
    /// Get all field names
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }
    
    /// Check if a field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }
    
    /// Get all fields as a vector
    pub fn fields(&self) -> Vec<FormField> {
        let mut result = Vec::new();
        
        for (name, values) in &self.fields {
            for value in values {
                result.push(FormField {
                    name: name.clone(),
                    value: value.clone(),
                });
            }
        }
        
        result
    }
    
    /// URL decode a string
    fn url_decode(input: &str) -> io::Result<String> {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '%' => {
                    // Get next two characters for hex code
                    let hex1 = chars.next().ok_or_else(|| io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid URL encoding: incomplete hex sequence"
                    ))?;
                    let hex2 = chars.next().ok_or_else(|| io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid URL encoding: incomplete hex sequence"
                    ))?;
                    
                    let hex_str = format!("{}{}", hex1, hex2);
                    let byte_val = u8::from_str_radix(&hex_str, 16)
                        .map_err(|_| io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid URL encoding: invalid hex sequence"
                        ))?;
                    
                    result.push(byte_val as char);
                }
                '+' => {
                    // Plus signs represent spaces in form data
                    result.push(' ');
                }
                _ => {
                    result.push(ch);
                }
            }
        }
        
        Ok(result)
    }
    
    /// URL encode a string
    pub fn url_encode(input: &str) -> String {
        let mut result = String::new();
        
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                b' ' => {
                    result.push('+');
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        
        result
    }
}

impl Default for FormData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_form_data_parsing() {
        let data = b"name=John+Doe&email=john%40example.com&age=30";
        let form_data = FormData::parse(data).unwrap();
        
        assert_eq!(form_data.get_field("name"), Some("John Doe"));
        assert_eq!(form_data.get_field("email"), Some("john@example.com"));
        assert_eq!(form_data.get_field("age"), Some("30"));
    }
    
    #[test]
    fn test_multiple_values() {
        let data = b"hobby=reading&hobby=coding&hobby=gaming";
        let form_data = FormData::parse(data).unwrap();
        
        let hobbies = form_data.get_field_values("hobby").unwrap();
        assert_eq!(hobbies.len(), 3);
        assert!(hobbies.contains(&"reading".to_string()));
        assert!(hobbies.contains(&"coding".to_string()));
        assert!(hobbies.contains(&"gaming".to_string()));
    }
    
    #[test]
    fn test_url_decode() {
        assert_eq!(FormData::url_decode("Hello%20World").unwrap(), "Hello World");
        assert_eq!(FormData::url_decode("test%40example.com").unwrap(), "test@example.com");
        assert_eq!(FormData::url_decode("no+encoding+needed").unwrap(), "no encoding needed");
    }
    
    #[test]
    fn test_url_encode() {
        assert_eq!(FormData::url_encode("Hello World"), "Hello+World");
        assert_eq!(FormData::url_encode("test@example.com"), "test%40example.com");
        assert_eq!(FormData::url_encode("simple"), "simple");
    }
    
    #[test]
    fn test_empty_form_data() {
        let data = b"";
        let form_data = FormData::parse(data).unwrap();
        assert_eq!(form_data.field_names().len(), 0);
    }
    
    #[test]
    fn test_field_without_value() {
        let data = b"submit&name=test";
        let form_data = FormData::parse(data).unwrap();
        
        assert_eq!(form_data.get_field("submit"), Some(""));
        assert_eq!(form_data.get_field("name"), Some("test"));
    }
}
