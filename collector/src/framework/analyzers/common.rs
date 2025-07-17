/// Common utilities for data analysis and processing across analyzers
use serde_json::Value;

/// Detect if string data is binary or text based on control characters
/// 
/// This function determines data type by checking if the string contains
/// control characters beyond the allowed ones (\n, \r, \t).
/// 
/// # Arguments
/// * `data_str` - The string data to analyze
/// 
/// # Returns
/// * `"text"` if the data contains only printable characters and allowed control chars
/// * `"binary"` if the data contains other control characters (likely binary data)
/// 
/// # Examples
/// ```
/// use framework::analyzers::common::detect_data_type;
/// 
/// assert_eq!(detect_data_type("Hello World"), "text");
/// assert_eq!(detect_data_type("HTTP/1.1 200 OK\r\n"), "text");
/// assert_eq!(detect_data_type("\x00\x01\x02binary"), "binary");
/// ```
pub fn detect_data_type(data_str: &str) -> &'static str {
    if data_str.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
        "text"
    } else {
        "binary"
    }
}

/// Convert data to a human-readable string representation
/// 
/// This function handles both text and binary data by detecting the type
/// and formatting appropriately. Binary data is converted to hex representation.
/// 
/// # Arguments
/// * `data` - The JSON value containing the data to convert
/// 
/// # Returns
/// * For text data: the original string
/// * For binary data: hex-encoded string with "HEX:" prefix
/// * For null data: "null" string
/// * For other types: JSON string representation
/// 
/// # Examples
/// ```
/// use serde_json::json;
/// use framework::analyzers::common::data_to_string;
/// 
/// let text_data = json!("Hello World");
/// assert_eq!(data_to_string(&text_data), "Hello World");
/// 
/// let binary_data = json!("\x00\x01\x02");
/// assert!(data_to_string(&binary_data).starts_with("HEX:"));
/// ```
pub fn data_to_string(data: &Value) -> String {
    match data {
        Value::String(s) => {
            // Check if string contains valid UTF-8 text or binary data
            if detect_data_type(s) == "text" {
                s.clone()
            } else {
                // Convert to hex if it contains control characters (likely binary)
                format!("HEX:{}", hex::encode(s.as_bytes()))
            }
        }
        Value::Null => "null".to_string(),
        _ => data.to_string()
    }
}

/// Check if a string contains only printable characters and allowed control characters
/// 
/// Allowed control characters are:
/// - '\n' (newline)
/// - '\r' (carriage return) 
/// - '\t' (tab)
/// 
/// # Arguments
/// * `s` - The string to check
/// 
/// # Returns
/// * `true` if the string is considered printable text
/// * `false` if the string contains other control characters (binary)
pub fn is_printable_text(s: &str) -> bool {
    detect_data_type(s) == "text"
}

/// Get a safe substring for display purposes
/// 
/// This function truncates long strings and handles binary data safely
/// by converting it to hex representation.
/// 
/// # Arguments
/// * `data_str` - The string data to process
/// * `max_length` - Maximum length for the result (default: 100)
/// 
/// # Returns
/// * Truncated and safely formatted string for display
pub fn safe_data_preview(data_str: &str, max_length: Option<usize>) -> String {
    let max_len = max_length.unwrap_or(100);
    
    let preview = if detect_data_type(data_str) == "text" {
        // For text data, just truncate if needed
        if data_str.len() > max_len {
            format!("{}...", &data_str[..max_len])
        } else {
            data_str.to_string()
        }
    } else {
        // For binary data, show hex representation
        let hex_str = hex::encode(data_str.as_bytes());
        if hex_str.len() > max_len {
            format!("HEX:{}...", &hex_str[..max_len])
        } else {
            format!("HEX:{}", hex_str)
        }
    };
    
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_data_type() {
        // Test text data detection
        assert_eq!(detect_data_type("Hello World"), "text");
        assert_eq!(detect_data_type("HTTP/1.1 200 OK\r\n"), "text");
        assert_eq!(detect_data_type("JSON: {\"key\": \"value\"}\n"), "text");
        assert_eq!(detect_data_type("Line1\nLine2\tTabbed"), "text");
        
        // Test binary data detection (contains control characters)
        assert_eq!(detect_data_type("\x00\x01\x02binary"), "binary");
        assert_eq!(detect_data_type("text\x00with\x01null"), "binary");
        assert_eq!(detect_data_type("\x1b[31mANSI\x1b[0m"), "binary");
        
        // Test edge cases
        assert_eq!(detect_data_type(""), "text"); // Empty string is text
        assert_eq!(detect_data_type("\r\n\t"), "text"); // Only allowed control chars
    }

    #[test]
    fn test_data_to_string() {
        // Test text data
        let text_value = json!("Hello World");
        assert_eq!(data_to_string(&text_value), "Hello World");
        
        // Test binary data
        let binary_value = json!("\x00\x01\x02binary");
        let result = data_to_string(&binary_value);
        assert!(result.starts_with("HEX:"));
        assert!(result.contains("000102"));
        
        // Test null value
        let null_value = json!(null);
        assert_eq!(data_to_string(&null_value), "null");
        
        // Test other types
        let number_value = json!(42);
        assert_eq!(data_to_string(&number_value), "42");
    }

    #[test]
    fn test_is_printable_text() {
        assert!(is_printable_text("Hello World"));
        assert!(is_printable_text("HTTP/1.1 200 OK\r\n"));
        assert!(!is_printable_text("\x00\x01\x02binary"));
        assert!(!is_printable_text("text\x00null"));
    }

    #[test]
    fn test_safe_data_preview() {
        // Test text data within limit
        assert_eq!(safe_data_preview("Hello", None), "Hello");
        
        // Test text data exceeding limit
        let long_text = "a".repeat(150);
        let preview = safe_data_preview(&long_text, Some(50));
        assert_eq!(preview.len(), 53); // 50 chars + "..."
        assert!(preview.ends_with("..."));
        
        // Test binary data
        let binary_data = "\x00\x01\x02\x03";
        let preview = safe_data_preview(binary_data, None);
        assert!(preview.starts_with("HEX:"));
        assert_eq!(preview, "HEX:00010203");
        
        // Test binary data exceeding limit
        let long_binary = "\x00".repeat(100);
        let preview = safe_data_preview(&long_binary, Some(20));
        assert!(preview.starts_with("HEX:"));
        assert!(preview.ends_with("..."));
        // HEX: prefix (4 chars) + 20 chars + ... (3 chars) = 27 chars
        assert_eq!(preview.len(), 27);
    }

    #[test]
    fn test_edge_cases() {
        // Empty string
        assert_eq!(detect_data_type(""), "text");
        assert_eq!(data_to_string(&json!("")), "");
        assert_eq!(safe_data_preview("", None), "");
        
        // Only control characters
        assert_eq!(detect_data_type("\r\n\t"), "text");
        assert_eq!(detect_data_type("\x00"), "binary");
        
        // Mixed content
        assert_eq!(detect_data_type("Hello\x00World"), "binary");
        assert_eq!(detect_data_type("Hello\nWorld"), "text");
    }
}