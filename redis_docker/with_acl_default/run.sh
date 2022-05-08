#!/usr/bin/env bash

set -ex

# ./run.sh 7.0-alpine 9999 "sleep 3"

version="${1:-7.0-alpine}"
listen_port=$2
callback=$3

if [ -z "$listen_port" ]
then
    exit 91
fi
if [ -z "$callback" ]
then
    exit 92
fi

script_path=$(cd $(dirname $0) ; pwd -P)
script_path_root="${script_path}/"

# 
container_name="redis_with_acl_default_${listen_port}"

conf_dir="${script_path_root}conf"

cleanup() {
    docker stop ${container_name}

    sleep 1
}
trap cleanup EXIT

docker run -d --rm --name ${container_name} \
    -v "${conf_dir}":/usr/local/etc/redis \
    -p ${listen_port}:6379\
    redis:${version} \
    redis-server /usr/local/etc/redis/redis.conf

sleep 1

if [ -x "$(command -v socat)" ]; then
    # https://www.compose.com/articles/how-to-talk-raw-redis/
    # https://gist.github.com/eeddaann/6e2b70e36f7586a556487f663b97760e
    { echo -e "*2\r\n\$4\r\nAUTH\r\n\$6\r\nmypass\r\n*1\r\n\$4\r\nINFO\r\n"; } | socat TCP4:127.0.0.1:${listen_port} stdio
fi

# 
echo "callback running..."
bash -c "${callback}"
