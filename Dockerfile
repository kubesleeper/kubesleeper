FROM busybox
RUN adduser --disabled-password ksuser
COPY ./target/x86_64-unknown-linux-musl/release/kubesleeper /usr/local/bin/kubesleeper
RUN chown ksuser:ksuser /usr/local/bin/kubesleeper
USER ksuser
CMD ["kubesleeper", "start"]
