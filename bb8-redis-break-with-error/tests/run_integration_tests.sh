#!/usr/bin/env bash

set -ex

version="${1:-7.0-alpine}"

# RUST_BACKTRACE=full ./tests/run_integration_tests.sh
# RUST_LOG=trace ./tests/run_integration_tests.sh

script_path=$(cd $(dirname $0) ; pwd -P)
script_path_root="${script_path}/"

run="${script_path_root}../../redis_docker/with_acl_default/run.sh"

# https://unix.stackexchange.com/questions/55913/whats-the-easiest-way-to-find-an-unused-local-port
read LOWERPORT UPPERPORT < /proc/sys/net/ipv4/ip_local_port_range
listen_port=$(comm -23 <(seq $LOWERPORT $UPPERPORT | sort) <(ss -Htan | awk '{print $4}' | cut -d':' -f2 | sort -u) | shuf | head -n 1)

export REDIS_TCP_PORT="${listen_port}"

${run} ${version} ${listen_port} "cd ${script_path_root}..; cargo test -p bb8-redis-break-with-error --features _integration_tests -- --nocapture"
