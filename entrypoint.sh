#!/bin/bash

set -xe

docker-entrypoint.sh mariadbd &
sudo -u mysql mocword-data
