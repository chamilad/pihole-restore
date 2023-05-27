BINARY="pihole_restore"
VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

.DEFAULT_GOAL: $(BINARY)

$(BINARY):
	cargo build --release

build-musl:
	cargo build --target x86_64-unknown-linux-musl

build-lowest-glibc:
	# buster at this point is on glibc 2.28
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -it rust:buster make

test: test-clean build-lowest-glibc
	mkdir -p test/pihole
	mkdir -p test/dnsmasq
	docker run --name pihole -d -v $(shell pwd)/test/pihole:/etc/pihole -v $(shell pwd)/test/dnsmasq:/etc/dnsmasq.d pihole/pihole:latest
	sleep 20
	docker cp ./target/release/pihole_restore pihole:./
	docker cp ./test/pi-hole_backup.tar.gz pihole:./
	docker exec -e RUST_LOG=debug -it pihole ./pihole_restore -f pi-hole_backup.tar.gz
	docker inspect pihole | grep IPAddress
	docker logs pihole 2>/dev/null | grep "Assigning random password"

test-clean:
	-docker stop pihole
	-docker rm pihole
	mkdir -p ./test/archive
	-sudo mv ./test/pihole ./test/archive/pihole-$(shell date +%Y-%m-%d_%H%M)
	-sudo mv ./test/dnsmasq ./test/archive/dnsmasq-$(shell date +%Y-%m-%d_%H%M)

release: build-lowest-glibc
	cp ./target/release/$(BINARY) ./target/release/$(BINARY)-$(VERSION)
