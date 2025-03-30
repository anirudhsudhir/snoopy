local-run:
	sudo LOG_LEVEL=TRACE cargo r -- 10.0.0.2 zeus-1:8000 chaos:8001

compress-send:
	tar -cvf snoopy.tar.gz ./src/ ./Cargo.lock ./Cargo.toml ./Makefile
	scp snoopy.tar.gz zeus-1:~/

# VM
zeus-1-run:
	cargo b
	sudo LOG_LEVEL=TRACE ./target/debug/snoopy 10.0.0.3 chaos:8001 zeus-1:8000
