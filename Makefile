.ONESHELL:
BINARY="pihole_restore"

.DEFAULT_GOAL: $(BINARY)

$(BINARY):
	cargo build --release
	chown -R $$(id -u):$$(id -g) target

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
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -t rust:buster make
	

build-lowest-glibc-arm:
	# buster at this point is on glibc 2.28
	docker run -v $(shell pwd):/usr/src/pihole_restore -w /usr/src/pihole_restore -t rust:buster make build-armv7

test: test-clean build-lowest-glibc
	mkdir -p test/pihole
	mkdir -p test/dnsmasq
	docker run --name test-pihole-test -d -v $(shell pwd)/test/pihole:/etc/pihole -v $(shell pwd)/test/dnsmasq:/etc/dnsmasq.d pihole/pihole:latest
	sleep 20
	docker cp ./target/release/pihole_restore test-pihole-test:./
	docker cp ./test/pi-hole_backup.tar.gz test-pihole-test:./
	docker exec -e RUST_LOG=debug -t test-pihole-test ./pihole_restore -f pi-hole_backup.tar.gz
	docker inspect test-pihole-test | grep IPAddress
	docker logs test-pihole-test 2>/dev/null | grep "Assigning random password"

# test an external binary, should be replaced with better testing
test-binary: test-clean
	mkdir -p test/pihole
	mkdir -p test/dnsmasq
	docker run --name test-pihole-test -d -v $(shell pwd)/test/pihole:/etc/pihole -v $(shell pwd)/test/dnsmasq:/etc/dnsmasq.d pihole/pihole:latest
	sleep 20
	docker cp ./target/external/pihole_restore test-pihole-test:./
	docker cp ./test/pi-hole_backup.tar.gz test-pihole-test:./
	docker exec -e RUST_LOG=debug -t test-pihole-test ./pihole_restore -f pi-hole_backup.tar.gz
	docker inspect test-pihole-test | grep IPAddress
	docker logs test-pihole-test 2>/dev/null | grep "Assigning random password"

test-clean:
	-docker stop test-pihole-test
	-docker rm test-pihole-test
	mkdir -p ./test/archive
	-sudo mv ./test/pihole ./test/archive/pihole-$(shell date +%Y-%m-%d_%H%M)
	-sudo mv ./test/dnsmasq ./test/archive/dnsmasq-$(shell date +%Y-%m-%d_%H%M)

clean:
	rm -rf target
