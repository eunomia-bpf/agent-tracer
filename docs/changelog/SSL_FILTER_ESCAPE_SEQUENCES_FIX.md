# SSL Filter Escape Sequences Fix

## Problem

The SSL filter command `--ssl-filter "data=0\r\n\r\n"` wasn't working because:

1. **Command line input**: `"data=0\r\n\r\n"` becomes literal `"data=0\\r\\n\\r\\n"` (backslash-r-backslash-n)
2. **Actual SSL data**: Contains real carriage return (`\r`) and newline (`\n`) characters
3. **No match**: `"0\\r\\n\\r\\n"` (9 chars) ≠ `"0\r\n\r\n"` (5 chars)

## Root Cause Analysis

```python
# What the command line gives us:
cmdline_input = "0\\r\\n\\r\\n"  # literal backslashes
print(repr(cmdline_input))      # '0\\r\\n\\r\\n' (9 characters)

# What's actually in the SSL data:
actual_data = "0\r\n\r\n"       # real CR/LF characters  
print(repr(actual_data))        # '0\r\n\r\n' (5 characters)

# Comparison:
print(cmdline_input == actual_data)  # False - they don't match!
```

## Solution

Added escape sequence processing to the SSL filter expression parser:

### 1. New Function: `process_escape_sequences()`

```rust
fn process_escape_sequences(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'r' => { chars.next(); result.push('\r'); }      // \r → CR
                    'n' => { chars.next(); result.push('\n'); }      // \n → LF  
                    't' => { chars.next(); result.push('\t'); }      // \t → TAB
                    '\\' => { chars.next(); result.push('\\'); }     // \\ → \
                    '"' => { chars.next(); result.push('"'); }       // \" → "
                    _ => result.push(ch),                            // Unknown escape
                }
            } else {
                result.push(ch);  // Backslash at end
            }
        } else {
            result.push(ch);      // Regular character
        }
    }
    result
}
```

### 2. Integration into Expression Parser

```rust
// In parse_condition(), process escape sequences:
let processed_value = Self::process_escape_sequences(&value);
return FilterNode::Condition { field, operator, value: processed_value };
```

### 3. Comprehensive Test Coverage

```rust
#[test]
fn test_escape_sequence_processing() {
    // Test basic escape sequences
    let processed = FilterExpression::process_escape_sequences("0\\r\\n\\r\\n");
    assert_eq!(processed, "0\r\n\r\n");
    
    // Test other escape sequences
    let processed2 = FilterExpression::process_escape_sequences("hello\\tworld\\n");
    assert_eq!(processed2, "hello\tworld\n");
    
    // Test with actual SSL data pattern
    let filter = FilterExpression::parse("data=0\\r\\n\\r\\n");
    
    let matching_event = json!({
        "data": "0\r\n\r\n",    // Real CR/LF characters
        "function": "READ/RECV",
        "len": 5
    });
    
    assert!(filter.evaluate(&matching_event));  // Now matches!
}
```

## Supported Escape Sequences

| Input | Output | Description |
|-------|--------|-------------|
| `\r` | `\r` | Carriage Return (CR) |
| `\n` | `\n` | Line Feed (LF) |
| `\t` | `\t` | Tab character |
| `\\` | `\` | Literal backslash |
| `\"` | `"` | Literal quote |

## Usage Examples

Now these commands work correctly:

```bash
# Filter connection teardown messages
cargo run ssl --ssl-filter "data=0\r\n\r\n"

# Filter HTTP responses with CRLF
cargo run ssl --ssl-filter "data~HTTP/1.1\r\n"

# Filter chunked encoding markers
cargo run ssl --ssl-filter "data=0\r\n\r\n&function=READ/RECV"

# Complex patterns with tabs and newlines
cargo run ssl --ssl-filter "data~Content-Type:\ttext/html\r\n"
```

## Before vs After

### Before (Broken):
```
Command: --ssl-filter "data=0\r\n\r\n"
Parser receives: "0\\r\\n\\r\\n" (literal backslashes)
SSL data contains: "0\r\n\r\n" (real CR/LF)
Result: No match ❌
```

### After (Fixed):
```
Command: --ssl-filter "data=0\r\n\r\n"  
Parser receives: "0\\r\\n\\r\\n" (literal backslashes)
Process escape sequences: "0\r\n\r\n" (real CR/LF)
SSL data contains: "0\r\n\r\n" (real CR/LF)
Result: Perfect match ✅
```

## Testing Results

```bash
$ cargo test ssl_filter --quiet
running 7 tests
.......
test result: ok. 7 passed; 0 failed; 0 ignored
```

All tests pass, including the new escape sequence test that specifically verifies the `data=0\r\n\r\n` pattern now works correctly.

## Benefits

1. **Intuitive Usage**: Users can now use familiar escape sequences like `\r\n`
2. **Backward Compatible**: Existing filters without escape sequences continue to work
3. **Comprehensive**: Supports all common escape sequences (`\r`, `\n`, `\t`, `\\`, `\"`)
4. **Robust**: Handles edge cases like trailing backslashes and unknown escape sequences
5. **Well Tested**: Comprehensive test coverage ensures reliability

The SSL filter now works exactly as expected with escape sequences, making it easy to filter SSL traffic patterns with special characters.