# --- Args
ARG TARGET=x86_64-unknown-linux-musl

# --- Build
FROM docker.io/rust:1.88 AS builder
ARG TARGET
WORKDIR /usr/src/kubesleeper

RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN wget https://github.com/tdewolff/minify/releases/download/v2.24.8/minify_linux_amd64.tar.gz && \
    tar -xzf minify_linux_amd64.tar.gz && \
    chmod +x minify

COPY static ./static
RUN ./minify static/waiting.html -o static/waiting.html

COPY Cargo.toml ./
COPY src ./src
RUN rustup target add x86_64-unknown-linux-musl
RUN RUSTFLAGS="-D warnings" cargo build --release --target x86_64-unknown-linux-musl --color always

# --- Export
# use docker|podman build --target binary-export --output type=local,dest=./dist .
# to build kubesleeper binary from dockerfile and get it locally
FROM scratch AS binary-export
ARG TARGET
COPY --from=builder /usr/src/kubesleeper/target/x86_64-unknown-linux-musl/release/kubesleeper /

# --- Image 
FROM busybox AS image
ARG TARGET
RUN adduser --disabled-password ksuser
COPY --from=builder /usr/src/kubesleeper/target/x86_64-unknown-linux-musl/release/kubesleeper /usr/local/bin/kubesleeper
RUN chown ksuser:ksuser /usr/local/bin/kubesleeper
USER ksuser
ENTRYPOINT ["kubesleeper"]
CMD ["start"]
