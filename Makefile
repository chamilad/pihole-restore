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

test: test-clean build-pihole
	mkdir -p test/pihole
	mkdir -p test/dnsmasq
	docker run --name pihole -d -v $(shell pwd)/test/pihole:/etc/pihole -v $(shell pwd)/test/dnsmasq:/etc/dnsmasq.d pihole/pihole:latest
	sleep 20
	docker logs pihole 2>/dev/null | grep "Assigning random password"
	docker cp ./target/release/pihole_restore pihole:./
	docker cp ./test/pi-hole_backup.tar.gz pihole:./
	docker exec -e RUST_LOG=debug -it pihole ./pihole_restore -f pi-hole_backup.tar.gz
	docker inspect pihole | grep IPAddress

test-clean:
	-docker stop pihole
	-docker rm pihole
	mkdir -p ./test/archive
	-sudo mv ./test/pihole ./test/archive/pihole-$(shell date +%Y-%m-%d_%H%M)
	-sudo mv ./test/dnsmasq ./test/archive/dnsmasq-$(shell date +%Y-%m-%d_%H%M)
