# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT** create a public GitHub issue for security vulnerabilities
2. Email security concerns to the maintainers privately
3. Include as much detail as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- Acknowledgment within 48 hours
- Regular updates on the status of your report
- Credit in the security advisory (if desired)

## Security Features

### Authentication

LMStudio Clone supports API key authentication to protect your endpoints:

```bash
# Single API key
export LMS_API_KEY="your-secret-key"

# Multiple API keys (comma-separated)
export LMS_API_KEYS="key1,key2,key3"
```

Usage:
```bash
# Using Authorization header
curl -H "Authorization: Bearer your-secret-key" http://localhost:1234/v1/models

# Using X-API-Key header
curl -H "X-API-Key: your-secret-key" http://localhost:1234/v1/models
```

### Rate Limiting

Protect against abuse with configurable rate limiting:

```bash
# Requests per minute (default: 60)
export LMS_RATE_LIMIT=100

# Burst size (default: 10)
export LMS_RATE_LIMIT_BURST=20
```

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Maximum requests per minute
- `X-RateLimit-Remaining`: Remaining requests in current window
- `Retry-After`: Seconds until rate limit resets (when limited)

### Network Binding

By default, the server binds to `127.0.0.1` (localhost only):

```bash
# Default: localhost only
lms-server --port 1234

# Enable network access (bind to 0.0.0.0)
lms-server --port 1234 --network
```

**Warning**: Only use `--network` in trusted network environments or behind a reverse proxy with proper authentication.

## Security Best Practices

### Production Deployment

1. **Always enable authentication**
   ```bash
   export LMS_API_KEY=$(openssl rand -hex 32)
   ```

2. **Use a reverse proxy** (nginx, Caddy, etc.) for:
   - TLS/HTTPS termination
   - Additional authentication layers
   - Request logging

3. **Enable rate limiting**
   ```bash
   export LMS_RATE_LIMIT=60
   export LMS_RATE_LIMIT_BURST=10
   ```

4. **Run with minimal privileges**
   ```bash
   # Create dedicated user
   useradd -r -s /bin/false lms

   # Run as non-root
   sudo -u lms lms-server
   ```

5. **Restrict file system access**
   - Only grant read access to model files
   - Use dedicated directories for uploads

### Docker Security

```dockerfile
# Run as non-root user
FROM rust:alpine
RUN adduser -D -u 1000 lms
USER lms

# Read-only root filesystem
docker run --read-only --tmpfs /tmp lms-server
```

### Reverse Proxy Configuration (nginx example)

```nginx
server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # Security headers
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;
    add_header X-XSS-Protection "1; mode=block";

    # Rate limiting at proxy level
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;

    location / {
        limit_req zone=api burst=20 nodelay;
        proxy_pass http://127.0.0.1:1234;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## Data Privacy

### Local-First Architecture

- All data stored locally on your machine
- No telemetry or external data collection
- Models run entirely on your hardware
- No API calls to external services

### Data Storage Locations

| Data Type | Default Location |
|-----------|------------------|
| Database | `~/.lmstudio-clone/lms.db` |
| Models | `~/.lmstudio-clone/models/` |
| Documents | `~/.lmstudio-clone/documents/` |

### Sensitive Data Handling

- API keys are stored in memory only
- Database uses SQLite with standard file permissions
- Uploaded documents are stored with restricted permissions

## Known Security Considerations

1. **Model Execution**: LLM models execute arbitrary computations. Only use models from trusted sources.

2. **Document Parsing**: PDF and DOCX parsing uses third-party libraries. Keep dependencies updated.

3. **Vector Storage**: Document embeddings are stored in SQLite. Consider encryption at rest for sensitive documents.

4. **CORS**: The server uses permissive CORS by default. Restrict origins in production.

## Security Changelog

### v1.0.0
- Added API key authentication
- Added rate limiting with token bucket algorithm
- Secure default: localhost-only binding
- Added security documentation
