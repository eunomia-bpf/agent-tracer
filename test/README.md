# Asset Loading Test Suite

This directory contains test scripts to debug asset loading issues in the AgentSight project.

## Files

- **`debug_assets.py`** - Python script that comprehensively tests asset loading
- **`test_assets.sh`** - Shell script wrapper for easy execution
- **`README.md`** - This file

## Usage

### Quick Test
```bash
./test_assets.sh
```

### Manual Python Test
```bash
python3 debug_assets.py
```

### Test with Custom Server URL
```bash
./test_assets.sh http://localhost:9000
python3 debug_assets.py http://localhost:9000
```

## What the Test Does

1. **Server Health Check** - Verifies the collector server is running
2. **Frontend Build Check** - Ensures frontend assets are built
3. **Root Endpoint Test** - Tests `/`, `/index.html`, `/timeline`
4. **Static Asset Test** - Tests CSS, JS, and other static files
5. **API Endpoint Test** - Tests available API endpoints

## Common Issues and Solutions

### "Asset not found" errors
1. Build the frontend: `cd frontend && npm run build`
2. Start the server: `cd collector && cargo run server`
3. Check embedded assets are properly included

### Server not responding
1. Ensure collector server is running: `cd collector && cargo run server`
2. Check port 8080 is available
3. Check firewall settings

### Frontend build issues
1. Install dependencies: `cd frontend && npm install`
2. Build frontend: `cd frontend && npm run build`
3. Check for TypeScript errors

## Output

The test script generates:
- Console output with test results
- `asset_debug_results.txt` file with detailed logs
- Status indicators: ✓ (success), ✗ (error), ? (warning)

## Integration with Development Workflow

Run this test script:
- After making changes to the frontend
- Before deploying the collector server
- When debugging asset loading issues
- As part of CI/CD pipeline validation