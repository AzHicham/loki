#!/bin/bash

# exit this script with failure on any error
set -e

function show_help() {
    cat << EOF
Usage: ${0##*/}  -o oauth_token
    -e      [push|pull_request]
    -b      [dev|release] if -e push, or the branch name if -e pull_request
    -f      navitia pull_request fork owner, needed only if -e pull_request
    -o      oauth token for github

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


while getopts "o:t:b:rp:u:e:f:h" opt; do
    case $opt in
        o) token=$OPTARG
            ;;
        b) branch=$OPTARG
            ;;
        u) user=$OPTARG
            ;;
        e) event=$OPTARG
            ;;
        f) fork=$OPTARG
            ;;
        h|\?)
            show_help
            exit 1
            ;;
    esac
done

if [[ -z $token ]] ;
then
     echo "Missing OAuth token for github. Specify one with -o token."
     exit 1
fi

if [[ -z $branch ]];
then
    echo "No branch given. I'll use branch = release  "
    branch="release"
fi

if [[ -z $event ]];
then
    echo "No event given. I'll use event = push  "
    event="push"
fi

if [[ $event == "push" ]]; then
    if [[ $branch == "dev" ]]; then
        workflow="build_navitia_packages_for_dev_multi_distribution.yml"
        archive="navitia-debian10-packages.zip"
        inside_archive="navitia_debian10_packages.zip"
    elif [[ $branch == "release" ]]; then
        workflow="build_navitia_packages_for_release.yml"
        archive="navitia-debian10-packages.zip"
        inside_archive="navitia_debian10_packages.zip"
    else
        echo """branch must be "dev" or "release" for push events (-e push)"""
        echo "***${branch}***"
        show_help
        exit 1
    fi
    fork="CanalTP"
elif [[ $event == "pull_request" ]]; then
    if [[ -z $branch ]]; then
        echo "branch must be set for pull_request events (-e pull_request -b branch_name)"
        show_help
        exit 1
    fi
    if [[ -z $fork ]]; then
        echo "fork must be set for pull_request events (-e pull_request -f fork)"
        show_help
        exit 1
    fi
    workflow="build_navitia_packages_for_dev_multi_distribution.yml"
    archive="navitia-debian10-packages.zip"
    inside_archive="navitia_debian10_packages.zip"
else
    echo """event must be "push" or "pull_request" """
    echo "***${event}***"
    show_help
    exit 1
fi


# fetch loki submodules
run git submodule update --init --recursive

# clone navitia source code with submodules
rm -rf ./tmp/
mkdir -p ./tmp/
run git clone https://x-token-auth:${token}@github.com/${fork}/navitia.git --branch $branch ./tmp/navitia/

# let's dowload the navitia package built on gihub actions
# for that we need the repo core_team_ci_tools
run git clone https://x-token-auth:${token}@github.com/CanalTP/core_team_ci_tools.git  ./tmp/core_team_ci_tools/

# we setup the right python environnement to use core_team_ci_tools
run pip install -r ./tmp/core_team_ci_tools/github_artifacts/requirements.txt --user

# let's download the navitia packages
run python ./tmp/core_team_ci_tools/github_artifacts/github_artifacts.py -o CanalTP -r navitia -t $token -w $workflow -b $branch -a $archive -e $event --output-dir ./tmp/ --waiting

# let's unzip what we received
run unzip -q ./tmp/${archive} -d ./tmp/

# let's unzip (again) to obtain the packages
run unzip -q ./tmp/${inside_archive} -d ./tmp/

# build the docker for binarisation
run docker build -f docker/bina_dockerfile -t navitia/mc_bina  .

# build the docker for kraken
run docker build -f docker/kraken_dockerfile -t navitia/mc_kraken  ./tmp/

# build the docker for jormun
run docker build -f docker/jormun_dockerfile -t navitia/mc_jormun  ./tmp/

# build the docker for server
run docker build -f docker/loki_dockerfile -t navitia/mc_loki  .


# clean up
rm -rf ./tmp/
