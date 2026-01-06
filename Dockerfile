# ---------- Stage 1: Build ----------
FROM rust:1.92 AS builder

WORKDIR /usr/src/app

# Install musl toolchain (NO OpenSSL)
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    build-essential \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and prefetch deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# Copy real source + sqlx metadata
COPY . .
COPY .sqlx .sqlx

# Enable SQLx offline mode
ENV SQLX_OFFLINE=true

# Add musl target
RUN rustup target add x86_64-unknown-linux-musl

# Tell Rust to use musl linker
ENV CC_x86_64_unknown_linux_musl=musl-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc

# Build fully static binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# ---------- Stage 2: Runtime ----------
FROM scratch

WORKDIR /usr/src/app

# Copy binary only
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/fileuploadservice .

EXPOSE 3000
CMD ["./fileuploadservice"]
