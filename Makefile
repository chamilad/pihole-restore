BINARY="pihole_restore"

.DEFAULT_GOAL: $(BINARY)

# run:
	# RUST_LOG=debug cargo run -- -f ./test/pi-hole_backup.tar.gz -d ./test/gravity.db -c

$(BINARY):
	# RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-gnu
	cargo build --release

build-musl:
	cargo build --target x86_64-unknown-linux-musl

build-pihole:
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -it rust:1-bullseye make
