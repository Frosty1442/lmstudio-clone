# Build stage
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    sqlite-dev \
    pkgconfig

WORKDIR /app

# Copy manifests
COPY lms-server/Cargo.toml lms-server/Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source code
COPY lms-server/src ./src
COPY lms-server/tests ./tests

# Build for release
RUN cargo build --release --locked

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    openssl \
    sqlite-libs \
    ca-certificates \
    tini

# Create non-root user
RUN addgroup -g 1000 lms && \
    adduser -D -u 1000 -G lms lms

# Create data directories
RUN mkdir -p /home/lms/.lmstudio-clone/models && \
    mkdir -p /home/lms/.lmstudio-clone/documents && \
    chown -R lms:lms /home/lms/.lmstudio-clone

# Copy binary from builder
COPY --from=builder /app/target/release/lms-server /usr/local/bin/lms-server

# Set permissions
RUN chmod +x /usr/local/bin/lms-server

# Switch to non-root user
USER lms
WORKDIR /home/lms

# Expose port
EXPOSE 1234

# Volume for data persistence
VOLUME ["/home/lms/.lmstudio-clone"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:1234/health || exit 1

# Use tini as init system
ENTRYPOINT ["/sbin/tini", "--"]

# Default command
CMD ["lms-server", "--port", "1234", "--network"]
