# Deployment Guide

This guide covers various deployment options for LMStudio Clone.

## Table of Contents

- [Quick Start](#quick-start)
- [Binary Installation](#binary-installation)
- [Docker Deployment](#docker-deployment)
- [Systemd Service](#systemd-service)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Reverse Proxy Setup](#reverse-proxy-setup)
- [Environment Variables](#environment-variables)
- [Monitoring](#monitoring)

## Quick Start

### From Source

```bash
# Clone repository
git clone https://github.com/your-org/lmstudio-clone.git
cd lmstudio-clone

# Build release binary
cargo build --release

# Run server
./target/release/lms-server --port 1234
```

### With Docker

```bash
docker run -d \
  -p 1234:1234 \
  -v ~/.lmstudio-clone:/data \
  -e LMS_API_KEY=your-secret-key \
  lmstudio-clone:latest
```

## Binary Installation

### Build Requirements

- Rust 1.70+
- OpenSSL development headers
- SQLite3

### Build Steps

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build optimized binary
cd lms-server
cargo build --release --locked

# Install system-wide
sudo cp target/release/lms-server /usr/local/bin/
```

### Command Line Options

```bash
lms-server --help

Options:
  -p, --port <PORT>         Port to listen on [default: 1234]
      --host <HOST>         Host to bind to [default: 127.0.0.1]
  -m, --models-dir <PATH>   Models directory
      --log-level <LEVEL>   Log level [default: info]
      --network             Allow network access (bind to 0.0.0.0)
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev sqlite-dev

WORKDIR /app
COPY . .
RUN cargo build --release --locked

FROM alpine:latest

RUN apk add --no-cache openssl sqlite-libs ca-certificates
RUN adduser -D -u 1000 lms

COPY --from=builder /app/target/release/lms-server /usr/local/bin/

USER lms
WORKDIR /home/lms

EXPOSE 1234
VOLUME ["/home/lms/.lmstudio-clone"]

ENTRYPOINT ["lms-server"]
CMD ["--port", "1234", "--network"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  lms-server:
    image: lmstudio-clone:latest
    container_name: lms-server
    restart: unless-stopped
    ports:
      - "1234:1234"
    volumes:
      - lms-data:/home/lms/.lmstudio-clone
      - ./models:/home/lms/.lmstudio-clone/models:ro
    environment:
      - LMS_API_KEY=${LMS_API_KEY}
      - LMS_RATE_LIMIT=100
      - LMS_RATE_LIMIT_BURST=20
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:1234/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  lms-data:
```

### Build and Run

```bash
# Build image
docker build -t lmstudio-clone:latest .

# Run with environment file
docker run -d \
  --name lms-server \
  --restart unless-stopped \
  -p 1234:1234 \
  -v lms-data:/home/lms/.lmstudio-clone \
  --env-file .env \
  lmstudio-clone:latest
```

## Systemd Service

### Service File

Create `/etc/systemd/system/lms-server.service`:

```ini
[Unit]
Description=LMStudio Clone Server
After=network.target

[Service]
Type=simple
User=lms
Group=lms
WorkingDirectory=/var/lib/lms
ExecStart=/usr/local/bin/lms-server --port 1234
Restart=always
RestartSec=5

# Environment
Environment=LMS_API_KEY=your-secret-key
Environment=LMS_RATE_LIMIT=100
EnvironmentFile=-/etc/lms-server/env

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/lms
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

[Install]
WantedBy=multi-user.target
```

### Setup

```bash
# Create service user
sudo useradd -r -s /bin/false -d /var/lib/lms lms
sudo mkdir -p /var/lib/lms
sudo chown lms:lms /var/lib/lms

# Install service
sudo systemctl daemon-reload
sudo systemctl enable lms-server
sudo systemctl start lms-server

# View logs
sudo journalctl -u lms-server -f
```

## Kubernetes Deployment

### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lms-server
  labels:
    app: lms-server
spec:
  replicas: 1
  selector:
    matchLabels:
      app: lms-server
  template:
    metadata:
      labels:
        app: lms-server
    spec:
      containers:
      - name: lms-server
        image: lmstudio-clone:latest
        ports:
        - containerPort: 1234
        env:
        - name: LMS_API_KEY
          valueFrom:
            secretKeyRef:
              name: lms-secrets
              key: api-key
        - name: LMS_RATE_LIMIT
          value: "100"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "4Gi"
            cpu: "4000m"
        volumeMounts:
        - name: data
          mountPath: /home/lms/.lmstudio-clone
        - name: models
          mountPath: /home/lms/.lmstudio-clone/models
          readOnly: true
        livenessProbe:
          httpGet:
            path: /health
            port: 1234
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 1234
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: lms-data
      - name: models
        persistentVolumeClaim:
          claimName: lms-models

---
apiVersion: v1
kind: Service
metadata:
  name: lms-server
spec:
  selector:
    app: lms-server
  ports:
  - port: 1234
    targetPort: 1234
  type: ClusterIP

---
apiVersion: v1
kind: Secret
metadata:
  name: lms-secrets
type: Opaque
stringData:
  api-key: "your-secret-key"
```

### GPU Support (NVIDIA)

```yaml
spec:
  containers:
  - name: lms-server
    resources:
      limits:
        nvidia.com/gpu: 1
```

## Reverse Proxy Setup

### Nginx

```nginx
upstream lms_backend {
    server 127.0.0.1:1234;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;

    # Security headers
    add_header X-Content-Type-Options nosniff always;
    add_header X-Frame-Options DENY always;
    add_header Strict-Transport-Security "max-age=31536000" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req zone=api burst=20 nodelay;

    location / {
        proxy_pass http://lms_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Streaming support
        proxy_buffering off;
        proxy_cache off;

        # Timeouts for long inference
        proxy_read_timeout 300s;
        proxy_connect_timeout 10s;
    }

    # Metrics endpoint (restrict access)
    location /metrics {
        allow 10.0.0.0/8;
        deny all;
        proxy_pass http://lms_backend;
    }
}
```

### Caddy

```caddyfile
api.example.com {
    reverse_proxy localhost:1234 {
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-For {remote_host}

        # Long timeout for inference
        transport http {
            read_timeout 300s
        }
    }

    # Restrict metrics
    @metrics path /metrics
    handle @metrics {
        respond "Forbidden" 403
    }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LMS_API_KEY` | - | Single API key for authentication |
| `LMS_API_KEYS` | - | Comma-separated list of API keys |
| `LMS_RATE_LIMIT` | 60 | Requests per minute per IP |
| `LMS_RATE_LIMIT_BURST` | 10 | Burst allowance |
| `RUST_LOG` | info | Log level (trace, debug, info, warn, error) |

## Monitoring

### Prometheus Integration

Metrics are exposed at `/metrics` in Prometheus format:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'lms-server'
    static_configs:
      - targets: ['localhost:1234']
    metrics_path: /metrics
    scheme: http
```

### Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `lms_uptime_seconds` | Gauge | Server uptime |
| `lms_requests_total` | Counter | Total requests by endpoint/status |
| `lms_request_duration_seconds` | Histogram | Request latency |
| `lms_models_loaded` | Gauge | Loaded model count |
| `lms_active_connections` | Gauge | Active connections |
| `lms_tokens_generated_total` | Counter | Total generated tokens |
| `lms_inference_duration_seconds` | Histogram | Inference latency |

### Grafana Dashboard

Import the provided dashboard from `monitoring/grafana-dashboard.json` or create panels for:

- Request rate and latency
- Error rate by endpoint
- Model loading status
- Token generation rate
- Resource utilization

### Health Checks

```bash
# Basic health check
curl http://localhost:1234/health

# Detailed status
curl http://localhost:1234/v1/status
```

## Troubleshooting

### Common Issues

1. **Port already in use**
   ```bash
   lsof -i :1234
   kill -9 <PID>
   ```

2. **Permission denied on models**
   ```bash
   chmod 755 ~/.lmstudio-clone/models
   chmod 644 ~/.lmstudio-clone/models/*.gguf
   ```

3. **Out of memory**
   - Use quantized models (Q4_K_M, Q5_K_S)
   - Limit context size
   - Add swap space

4. **Slow inference**
   - Enable GPU acceleration
   - Use smaller models
   - Reduce batch size

### Debug Mode

```bash
RUST_LOG=debug lms-server --port 1234
```
