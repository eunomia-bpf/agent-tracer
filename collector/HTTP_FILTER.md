# HTTP Filter Documentation

The HTTP Filter is a powerful analyzer that filters HTTP parser events based on configurable expressions. It's similar to the Python `filter_expression.py` but integrated into the Rust collector framework.

## Overview

The HTTP Filter allows you to exclude specific HTTP requests and responses from the event stream based on various criteria such as:
- Request method, path, headers, and body content
- Response status codes, headers, and body content
- Complex logical expressions with AND/OR operators

## Command Line Usage

```bash
# Basic usage with HTTP parsing and filtering
./collector ssl --http-parser --http-filter "request.path_prefix=/v1/health"

# Multiple filter patterns
./collector ssl --http-parser --http-filter "request.method=GET" --http-filter "response.status_code=404"

# Complex expressions with logical operators
./collector ssl --http-parser --http-filter "request.method=GET | response.status_code=404"
./collector ssl --http-parser --http-filter "request.path_prefix=/v1/rgstr & response.status_code=202"
```

## Filter Expression Syntax

### Basic Syntax

Filter expressions use dot notation to specify the target (request/response) and field:

```
target.field=value
```

### Targets

- `request` or `req` - Filter HTTP requests
- `response` or `resp` or `res` - Filter HTTP responses

### Request Fields

| Field | Description | Example |
|-------|-------------|---------|
| `method` | HTTP method | `request.method=GET` |
| `path` | Request path (exact match) | `request.path=/api/users` |
| `path_prefix` | Path starts with | `request.path_prefix=/v1/` |
| `path_contains` | Path contains | `request.path_contains=health` |
| `host` | Host header | `request.host=api.example.com` |
| `body` or `body_contains` | Request body contains text | `request.body={"user_id"` |

### Response Fields

| Field | Description | Example |
|-------|-------------|---------|
| `status_code` | HTTP status code | `response.status_code=404` |
| `status_text` | Status text contains | `response.status_text=Not Found` |
| `content_type` | Content-Type header contains | `response.content_type=application/json` |
| `server` | Server header contains | `response.server=nginx` |
| `body` or `body_contains` | Response body contains text | `response.body=error` |

### Logical Operators

- `&` - AND operator (higher precedence)
- `|` - OR operator (lower precedence)

### Legacy Syntax (Backward Compatibility)

For backward compatibility, the following legacy formats are supported:

```bash
# Legacy path filtering (assumes request.path_contains)
--http-filter "/health"

# Legacy field=value (assumes request target)
--http-filter "method=GET"
--http-filter "path_prefix=/v1/"
```

## Examples

### Basic Request Filtering

```bash
# Filter out health check requests
./collector ssl --http-parser --http-filter "request.path_prefix=/health"

# Filter out GET requests
./collector ssl --http-parser --http-filter "request.method=GET"

# Filter out requests to specific host
./collector ssl --http-parser --http-filter "request.host=api.internal.com"
```

### Basic Response Filtering

```bash
# Filter out 404 responses
./collector ssl --http-parser --http-filter "response.status_code=404"

# Filter out successful responses
./collector ssl --http-parser --http-filter "response.status_code=200"

# Filter out JSON responses
./collector ssl --http-parser --http-filter "response.content_type=application/json"
```

### Body Filtering

The HTTP filter supports filtering based on request and response body content using substring matching:

#### Request Body Filtering

```bash
# Filter out requests with empty body
./collector ssl --http-parser --http-filter "request.body="

# Filter out requests containing specific JSON
./collector ssl --http-parser --http-filter "request.body={\"action\":\"ping\"}"

# Filter out requests with specific API keys
./collector ssl --http-parser --http-filter "request.body=api_key"

# Filter out login requests
./collector ssl --http-parser --http-filter "request.body=password"

# Filter out specific user operations
./collector ssl --http-parser --http-filter "request.body=user_id\":\"12345"
```

#### Response Body Filtering

```bash
# Filter out responses with empty body
./collector ssl --http-parser --http-filter "response.body="

# Filter out successful API responses
./collector ssl --http-parser --http-filter "response.body={\"status\":\"success\"}"

# Filter out error responses containing specific messages
./collector ssl --http-parser --http-filter "response.body=Internal Server Error"

# Filter out debug responses
./collector ssl --http-parser --http-filter "response.body=debug"

# Filter out large JSON responses with specific structure
./collector ssl --http-parser --http-filter "response.body=data\":[{"
```

#### Advanced Body Filtering

```bash
# Combine body and header filtering
./collector ssl --http-parser --http-filter "request.body=api_key & request.host=internal.api.com"

# Filter requests with empty body OR specific content
./collector ssl --http-parser --http-filter "request.body= | request.body=keepalive"

# Filter responses based on body content AND status
./collector ssl --http-parser --http-filter "response.body=error & response.status_code=500"

# Filter out authentication flows
./collector ssl --http-parser --http-filter "request.body=username | request.body=token | response.body=access_token"
```

### Complex Expressions

```bash
# Filter out GET requests OR 404 responses
./collector ssl --http-parser --http-filter "request.method=GET | response.status_code=404"

# Filter out health checks AND successful responses
./collector ssl --http-parser --http-filter "request.path_prefix=/health & response.status_code=200"

# Filter out internal API requests with specific status
./collector ssl --http-parser --http-filter "request.host=api.internal.com & response.status_code=401"
```

### Multiple Filter Patterns

```bash
# Multiple patterns are combined with OR logic
./collector ssl --http-parser \
  --http-filter "request.path_prefix=/v1/health" \
  --http-filter "request.path_prefix=/v1/status" \
  --http-filter "response.status_code=200"
```

### Real-World Examples

```bash
# Hide monitoring and health check traffic
./collector ssl --http-parser \
  --http-filter "request.path_prefix=/health" \
  --http-filter "request.path_prefix=/metrics" \
  --http-filter "request.path_prefix=/status"

# Focus on error responses only
./collector ssl --http-parser \
  --http-filter "response.status_code=200" \
  --http-filter "response.status_code=201" \
  --http-filter "response.status_code=204"

# Hide static asset requests
./collector ssl --http-parser \
  --http-filter "request.path_contains=.js" \
  --http-filter "request.path_contains=.css" \
  --http-filter "request.path_contains=.png" \
  --http-filter "request.path_contains=.ico"

# Filter out OPTIONS requests and their responses
./collector ssl --http-parser \
  --http-filter "request.method=OPTIONS"
```

## Integration with Other Analyzers

The HTTP Filter works as part of the analyzer chain and should be placed after the HTTP Parser:

```
SSL Data → SSE Processor → HTTP Parser → HTTP Filter → File Logger → Output
```

Example command:
```bash
./collector ssl --http-parser --http-filter "request.path_prefix=/internal" --quiet
```

This will:
1. Capture SSL traffic
2. Process Server-Sent Events (SSE)
3. Parse HTTP requests/responses
4. Filter out requests starting with `/internal`
5. Log filtered results to file
6. Suppress console output

## Performance Considerations

- HTTP filtering is lightweight and processes events in streaming fashion
- Complex expressions with many OR conditions may have slight performance impact
- Filter expressions are compiled once at startup for efficiency
- Non-HTTP events (raw SSL data, SSE processor events) pass through unchanged

## Error Handling

- Invalid filter expressions are ignored with warnings
- Malformed HTTP data that can't be parsed is passed through unchanged
- The analyzer gracefully handles missing fields in HTTP data

## Debugging

Filter expressions include debug support. Currently, debug mode is not exposed via command line but can be enabled programmatically:

```rust
let filter = HTTPFilter::with_patterns(patterns).with_debug();
```

This will print detailed information about which events are being filtered and why.

## Technical Details

### Event Processing

The HTTP Filter only processes events with `source = "http_parser"`. Other events (SSL, SSE, etc.) pass through unchanged.

### Filter Logic

1. Parse filter expressions into a tree structure with logical operators
2. For each HTTP parser event:
   - Check if event matches request/response target
   - Evaluate conditions against HTTP fields
   - Apply logical operators (AND/OR)
   - Filter out events that match any expression

### Thread Safety

The HTTP Filter is designed to be thread-safe and can be used in concurrent analyzer chains.

### Body Filtering Implementation

Body filtering uses substring matching with the following characteristics:

1. **Request Body**: Accessed via `data.body` field in HTTP parser events
2. **Response Body**: Accessed via `data.body` field in HTTP parser events  
3. **Matching**: Uses `contains()` method for substring matching
4. **Empty Body**: Use `request.body=` or `response.body=` to match empty bodies
5. **Case Sensitivity**: Body filtering is case-sensitive
6. **Performance**: Body filtering processes the entire body content in memory

#### Body Filtering Limitations

- **Memory Usage**: Large bodies are held in memory during filtering
- **Binary Data**: Only works with UTF-8 text content, binary data may cause issues
- **Partial Content**: If body is truncated due to size limits, filtering may not work as expected
- **Streaming**: Body must be complete before filtering can occur

## Troubleshooting

### Common Issues

1. **Filter not working**: Ensure you're using `--http-parser` flag
2. **No events shown**: Check if your filter expressions are too broad
3. **Syntax errors**: Verify dot notation syntax (e.g., `request.path=value`)
4. **Body filtering not working**: Check if HTTP parser is capturing body content
5. **Empty body filter issues**: Use exact `request.body=` syntax for empty bodies
6. **Binary content issues**: Body filtering only works with UTF-8 text content

### Validation

Test your filter expressions with a small dataset first:

```bash
# Test with verbose output
./collector ssl --http-parser --http-filter "request.method=GET" -p 12345
```

### Debugging Tips

1. Start with simple expressions and add complexity gradually
2. Use multiple specific patterns rather than complex OR expressions
3. Check that field names match exactly (case-sensitive)
4. Remember that filtering excludes matching events (blacklist approach)
