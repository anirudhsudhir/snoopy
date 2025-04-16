default:
  just --list

build-linux-aarch64:
  cargo b --target aarch64-unknown-linux-musl

build-image: build-linux-aarch64
  docker build --platform linux/aarch64 -t snoopy_test .

exec-peer-1:
  docker compose exec peer_1 sh

exec-peer-2:
  docker compose exec peer_2 sh

exec-peer-3:
  docker compose exec peer_3 sh
