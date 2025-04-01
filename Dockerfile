# Use the latest stable Rust image
FROM rust:latest as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy the entire project
COPY . .

# Build the application
RUN cargo build --release

# Find and rename the binary to a known name
RUN find /app/target/release -type f -executable -not -name "*.so" -not -name "*.a" \
    -not -path "*/deps/*" -not -name "build-script-*" | head -n 1 | xargs -I {} cp {} /app/rust-application

# Use Ubuntu 22.04 as runtime base which includes OpenSSL 3
FROM ubuntu:22.04

WORKDIR /app

# Install runtime dependencies (OpenSSL 3 and PostgreSQL client libraries)
RUN apt-get update && \
    apt-get install -y libssl3 libpq5 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/rust-application /app/rust-application

COPY .env .env

COPY mux.pem mux.pem
RUN chmod 600 mux.pem

# Expose the port
EXPOSE 8080

# Start the application
CMD ["/app/rust-application"]