# pihole-restore

`pihole_restore` is a CLI tool to restore a Pihole Backup archive file. It can
be used in a setup where multiple Pihole servers are configured that needs
syncing from a given master configuration point. This CLI is intended to be run
on the same runtime as the target Pihole server itself. It cannot run in a
detached runtime, since it depends on being able to access the `pihole` CLI.

![design](./img/pihole-restore-design.png)

![restore](./img/restore.png)

The motivation to write this tool was the absence of a `restore` command in the
`pihole` CLI shipped with Pi-hole.

## Compile Time Dependencies

1. `glibc` - At most v2.28 (since the oldest pihole setup I could get my hands
   on had this version)

### Ubuntu

The following dev headers will be needed to compile the binary.

1. `libsqlite3-dev`

## Runtime Dependencies

1. `glibc` - At least v2.28 (if your runtime is older than this, please open an
   issue. I haven't explored far back enough to see if the older docker images
   have runtimes older than this.)

## Usage

```
$ pihole_restore -h
Usage: pihole_restore [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>          teleporter archive file to restore from
  -d, --database <DATABASE>  gravity db file location [default: /etc/pihole/gravity.db]
  -c, --clear                clean existing tables and files
      --filters <FILTERS>    filter which config to restore, specify in comma separated keywords [default: all]
  -h, --help                 Print help
  -V, --version              Print version
```

In a typical scenario, the following command will restore from the archive to
the `/etc/pihole/gravity.db` and the other locations inside Pihole
installation.

```
pihole_restore -f pihole-backup.tar.gz
```

If the current configuration needs to be cleared before restoring from the
archive, `-c` (`--clear`) flag can be used. Doing so will clear out the tables
and the configuration files.

By default, the following configuration is restored.

1. `blacklist` - Blacklist (exact)
1. `blacklistregex` - Blacklist (regex)
1. `whitelist` - Whitelist (exact)
1. `whitelistregex` - Whitelist (regex)
1. `adlist` - Adlists
1. `auditlog` - Audit log
1. `group` - Group
1. `client` - Client
1. `staticdhcp` - Static DHCP Leases
1. `localdns` - Local DNS Records
1. `localcname` - Local CNAME Records

If only a subset of this configuration needs to be applied, use the `--filter`
argument. For an example, to restore only the Local DNS records run,

```
pihole_restore -f <archive_file.tar.gz> --filter localdns
```

> use `sudo` in Raspbian since Pihole runs as `pihole` `nologin` user

Multiple filters can be specified as a comma separated string.

```
# restore blacklist (exact), adlists, groups, and clients
pihole_restore -f <archive_file.tar.gz> --filter blacklist,adlist,group,client
```

## TODO

1. test more use cases
1. automated testing to cover most code
1. edge cases on deduplication
1. support for older glibc runtimes
1. possibility on a fully independent binary (musl C?)

## Development

Use the `makefile` target `test` to spin up a pihole Docker container. It
compiles the binary on a Debian Buster container (to link to glibc 2.28), 
copies the result to the Pihole container, copies the sample backup archive to
the container, and runs a restore job. The directories `test/pihole` and
`test/dnsmasq` are mounted to `/etc/pihole` and `/etc/dnsmasq.d` respectively.

So the sample archive should exist in the `test` directory before running `make
test`. For this, create a backup of a Pihole setup
and copy it to the `test` directory as `test/pi-hole_backup.tar.gz`.

At the end of the run, `make test` will output the IP address of the pihole
container and the randomly generated admin password from the logs.

`make test-clean` will stop and clean the test container (named
`test-pihole-test`, so make sure you don't have any existing containers named
with that ID), and move the `pihole` and `dnsmasq` directories to the
`test/archive` directory for post-test analysis.

## License

This source code and the binary is licensed under Apache v2 license. It is not
distributed with the official Pihole distribution, and it is not endorsed by
official Pi-hole community at this moment.
