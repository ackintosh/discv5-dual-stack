#!/bin/bash

set -ex

ip address show eth0

until redis-cli -h redis ping > /dev/null; do
  echo "Redis is not available - sleeping"
  sleep 1
done

RUST_LOG=discv5=trace /usr/local/bin/discv5-dual-stack "$1"
