.ONESHELL:
BINARY="pihole_restore"

.DEFAULT_GOAL: $(BINARY)

$(BINARY):
	cargo build --release
	chown -R 1000:1000 target

build-armv7:
	# add linker config
	echo "[target.armv7-unknown-linux-gnueabihf]" > /usr/local/cargo/config
	echo "linker = \"arm-linux-gnueabihf-gcc\"" >> /usr/local/cargo/config
	cat /usr/local/cargo/config
	# add architecture
	dpkg --add-architecture armhf
	apt-get update
	apt-get install -y curl git build-essential
	apt-get install -y libc6-armhf-cross libc6-dev-armhf-cross gcc-arm-linux-gnueabihf
	# we use sqlite
	apt-get install -y libsqlite3-0:armhf libsqlite3-dev:armhf
	rustup default stable
	rustup target add x86_64-unknown-linux-gnu
	rustup target add armv7-unknown-linux-gnueabihf
	export PKG_CONFIG_PATH="/usr/lib/arm-linux-gnueabihf/pkgconfig"
	export PKG_CONFIG_ALLOW_CROSS="true"
	# build
	cargo build --release --target armv7-unknown-linux-gnueabihf
	chown -R 1000:1000 target

build-lowest-glibc:
	# buster at this point is on glibc 2.28
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -it rust:buster make
	

build-lowest-glibc-arm:
	# buster at this point is on glibc 2.28
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -it rust:buster make build-armv7

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

release: clean build-lowest-glibc
	cp ./target/release/$(BINARY) ./target/release/$(BINARY)-$(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')-x86_64

release-arm: clean build-lowest-glibc-arm
	cp ./target/armv7-unknown-linux-gnueabihf/release/$(BINARY) ./target/armv7-unknown-linux-gnueabihf/release/$(BINARY)-$(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')-armv7

clean:
	rm -rf target
