# TODO: Build the binary and copy it to a minimal image using a multi-stage build

FROM alpine

WORKDIR /usr/app/snoopy

RUN apk update
RUN apk add netcat-openbsd python3 curl

COPY target/aarch64-unknown-linux-musl/debug/snoopy .

ENV LOG_LEVEL=TRACE
CMD ["./snoopy","config.toml"]
