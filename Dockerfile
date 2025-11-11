FROM docker.io/rust:1.86 AS builder
WORKDIR /usr/src/kubesleeper

COPY static ./static
RUN cargo install minhtml
RUN /usr/local/cargo/bin/minhtml --minify-css --minify-js static/waiting.html -o static/waiting.html

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
COPY src ./src
RUN cargo build --release

FROM busybox
RUN adduser --disabled-password ksuser
COPY --from=builder /usr/src/kubesleeper/target/release/kubesleeper /usr/local/bin/kubesleeper
RUN chown ksuser:ksuser /usr/local/bin/kubesleeper
USER ksuser
CMD ["kubesleeper"]
