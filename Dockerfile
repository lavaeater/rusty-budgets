# Build stage
FROM rust:slim-bullseye AS builder
LABEL authors="tommie"

WORKDIR /usr/src/pod-crab
COPY . .

RUN apt-get update && apt-get install -y apt-utils pkg-config libssl-dev openssl curl

# Build the root package (not a specific workspace member)
RUN cargo install --path .

# Install Node.js and Rspack
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs 


WORKDIR /usr/src/pod-crab/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm install

# Copy frontend source and build it
COPY frontend ./
RUN npm run build  # This assumes `build` runs `rspack build`

# Runtime stage
FROM debian:bullseye-slim

# Install necessary runtime dependencies (if any)
RUN apt-get update && apt-get install -y apt-utils pkg-config libssl-dev openssl ca-certificates && apt-get clean && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/local/cargo/bin/pod-crab /usr/local/bin/pod-crab

COPY --from=builder /usr/src/pod-crab/frontend/dist /usr/local/bin/frontend/dist
COPY --from=builder /usr/src/pod-crab/static /usr/local/bin/static
COPY --from=builder /usr/src/pod-crab/frontend/templates /usr/local/bin/frontend/templates

# Ensure the working directory is set (useful for relative file paths)
WORKDIR /usr/local/bin

# Start the application
CMD ["/usr/local/bin/pod-crab"]
