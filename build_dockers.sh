#!/bin/bash

# exit this script with failure on any error
set -e

function show_help() {
    cat << EOF
Usage: ${0##*/}
    -t      navitia docker tag, default to latest
EOF
}

#we want to be able to interupt the build, see: http://veithen.github.io/2014/11/16/sigterm-propagation.html
function run() {
    trap 'kill -TERM $PID' TERM INT
    $@ &
    PID=$!
    wait $PID
    trap - TERM INT
    wait $PID
    return $?
}


while getopts "t:" opt; do
    case $opt in
        t) tag=$OPTARG
            ;;
        h|\?)
            show_help
            exit 1
            ;;
    esac
done

if [[ -z $tag ]];
then
    echo "No tag given for navitia dockers. I'll use tag = latest"
    tag="latest"
fi

# fetch loki submodules
run git submodule update --init --recursive

# build the docker for binarisation
run docker build -f docker/bina_dockerfile -t navitia/mc_bina --build-arg NAVITIA_TAG=${tag} .

# build the docker for kraken
run docker build -f docker/kraken_dockerfile -t navitia/mc_kraken --build-arg NAVITIA_TAG=${tag} .

# build the docker for jormun
run docker build -f docker/jormun_dockerfile -t navitia/mc_jormun --build-arg NAVITIA_TAG=${tag} .

# build the docker for server
run docker build -f docker/loki_dockerfile -t navitia/mc_loki .

# build the docker for server
run docker build -f docker/loki_aws_dockerfile -t navitia/loki .
