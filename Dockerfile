FROM rust:1.85-alpine

RUN apk add alpine-sdk
RUN apk add netcat-openbsd
RUN apk add python3
