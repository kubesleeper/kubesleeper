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

RUN cargo install minhtml --color always

COPY static ./static
RUN /usr/local/cargo/bin/minhtml --minify-css --minify-js static/waiting.html -o static/waiting.html

<<<<<<< HEAD
COPY Cargo.toml ./
COPY src ./src
RUN rustup target add x86_64-unknown-linux-musl
=======
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build --release --target x86_64-unknown-linux-musl

COPY src ./src
>>>>>>> a2e0969 (feat: smart parsing + doc + ks checks + cli rework)
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
