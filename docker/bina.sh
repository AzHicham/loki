#!/bin/bash
set -e

input="/storage/data"
output="/storage/mc_navitia"

rm -rf ${output}
mkdir -p ${output}
chmod -R 777 ${output}

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


# we initialize the docker-compose.yml with the 
# services used for all coverages
echo """
version: \"3\"
services: 
  jormungandr:
    image: mc_navitia/jormun
    volumes:
      - .:/data
    ports:
      - 9191:80
  
  rabbitmq:
    image: rabbitmq:management
""" > ${output}/docker-compose.yml

mkdir -p ${output}/jormun_conf/

cd ${input}
for folder in $(ls -d */); do
    coverage=${folder%%/}
    echo "Configuring ${coverage}"

    if [[ ! -e ${input}/${coverage}/gtfs/ ]] && [[ ! -e ${input}/${coverage}/ntfs/ ]]; then
      echo "No gtfs/ nor ntfs/ subdirectory found in ${input}/${coverage}."
      echo "I skip coverage ${coverage}."
      continue
    fi

    if [[ -e ${input}/${coverage}/gtfs/ ]] && [[ -e ${input}/${coverage}/ntfs/ ]]; then
      echo "Found both gtfs/ nor ntfs/ subdirectory in ${input}/${coverage}."
      echo "I don't know which one to use so I skip the coverage ${coverage}."
      continue
    fi


    mkdir -p ${output}/${coverage}

    # copy gtfs data to output if present
    if [[ -e ${input}/${coverage}/gtfs/ ]]; then
      
      rm -f ${output}/${coverage}/gtfs/*
      mkdir -p ${output}/${coverage}/gtfs/
      cp  ${input}/${coverage}/gtfs/* ${output}/${coverage}/gtfs/

      # remove "StopPoint:" prefix on stop point uris'
      sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/stops.txt
      sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/stop_times.txt
      sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/transfers.txt
    fi

    # copy ntfs data to output if present
    if [[ -e ${input}/${coverage}/ntfs/ ]]; then
      
      rm -f ${output}/${coverage}/ntfs/*
      mkdir -p ${output}/${coverage}/ntfs/
      cp  ${input}/${coverage}/ntfs/* ${output}/${coverage}/ntfs/
    fi

    # copy osm data to output if present
    if [[ -e ${input}/${coverage}/osm/ ]]; then
        rm -f ${output}/${coverage}/osm/*
        mkdir -p ${output}/${coverage}/osm/
        cp  ${input}/${coverage}/osm/* ${output}/${coverage}/osm/
    fi    

    # binarize
    echo "Launch binarisation"
    rm -f ${output}/${coverage}/data.nav.lz4
    run python3 /navitia/source/eitri/eitri.py -d ${output}/${coverage}/ -e /usr/bin -o ${output}/${coverage}/data.nav.lz4

    # copy stoptime_loads
    cp ${input}/${coverage}/stoptimes_loads.csv ${output}/${coverage}/stoptimes_loads.csv 

    # if gtfs was given as input, we transform it into gtfs for feeding loki
    if [[ -e ${output}/${coverage}/gtfs/ ]]; then
      #tranform gtfs into ntfs
      echo "Launch gtfs2ntfs"
      rm -f ${output}/${coverage}/ntfs/*
      mkdir -p ${output}/${coverage}/ntfs
      run gtfs2ntfs --input ${output}/${coverage}/gtfs --output ${output}/${coverage}/ntfs
    fi

    # add kraken and loki services for this coverage
    echo """
  loki-${coverage}:
    image: mc_navitia/loki
    environment: 
      - RUST_LOG=debug
    volumes:
      - ./${coverage}/:/data

  kraken-${coverage}:
    image: mc_navitia/kraken:latest
    volumes:
      - ./${coverage}:/data
""" >> ${output}/docker-compose.yml



    krakenPort="30000"
    lokiBasicPort="30001"
    lokiLoadsPort="30002"

    # Jormun config files
    # one for the "kraken" coverage
    jq -n --arg instance "${coverage}-kraken" --arg krakenSocket "tcp://kraken-${coverage}:${krakenPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket
}'  > ${output}/jormun_conf/$coverage.json

    # one for "loki" with loads comparator
    jq -n --arg instance "${coverage}-loki-loads" --arg krakenSocket "tcp://kraken-${coverage}:${krakenPort}" --arg lokiSocket "tcp://loki-${coverage}:${lokiLoadsPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $lokiSocket 
}'  > ${output}/jormun_conf/${coverage}_loads.json

    # one for "loki" with basic comparator
    jq -n --arg instance "${coverage}-loki-basic" --arg krakenSocket "tcp://kraken-${coverage}:${krakenPort}" --arg lokiSocket "tcp://loki-${coverage}:${lokiBasicPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $lokiSocket 
}'  > ${output}/jormun_conf/${coverage}_classic.json

    # kraken config file
    echo "[GENERAL]
instance_name = ${coverage}-kraken
database = /data/data.nav.lz4
zmq_socket = tcp://*:${krakenPort}

[BROKER]
host = rabbitmq
port = 5672
username = guest
password = guest
" > ${output}/${coverage}/kraken.ini

    # Loki config files
    # one for the coverage with loads criteria
    jq -n --arg basicSocket "tcp://*:$lokiBasicPort" --arg loadsSocket "tcp://*:$lokiLoadsPort" '{
    ntfs_path: "/data/ntfs/",
    loads_data_path: "/data/stoptimes_loads.csv",
    basic_requests_socket: $basicSocket,
    loads_requests_socket: $loadsSocket,
    data_implem: "loads_periodic",
    criteria_implem: "loads"
}' > ${output}/${coverage}/loki_config.json



    echo "${coverage} done"
done


chmod -R 777 ${output}