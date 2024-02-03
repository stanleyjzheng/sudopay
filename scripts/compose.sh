#!/usr/bin/env sh

find_cmd() {
	which "$1" >/dev/null 2>&1
}

if $(find_cmd docker-compose); then
	CMD="docker-compose"
elif $(find_cmd podman-compose); then
	CMD="podman-compose"
else
	echo "ERROR: Either docker-compose or podman-compose is required start the server"
	exit 1
fi

trap 'kill $SOLID && docker-compose down && echo Killed' INT

"${CMD}" up -d