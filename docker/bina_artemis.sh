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
    image: navitia/mc_jormun
    volumes:
      - .:/data
    ports:
      - 9191:80

""" > ${output}/docker-compose.yml


# we initialize the kubernetes.yml with the
# services used for all coverages
echo """
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: volume-claim-navitia
spec:
  storageClassName: storage-class-navitia
  accessModes:
    - ReadOnlyMany
  resources:
    requests:
      storage: 100Mi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: deployment-jormun
spec:
  replicas: 1
  selector:
    matchLabels:
      app: app-jormun
  template:
    metadata:
      labels:
        app : app-jormun
    spec:
      containers:
        - image: navitia/mc_jormun
          name: jormungandr
          ports:
            - containerPort: 80
          volumeMounts:
            - name: volume-navitia
              mountPath: /data
      volumes:
        - name: volume-navitia
          persistentVolumeClaim:
            claimName: volume-claim-navitia
---
apiVersion: v1
kind: Service
metadata:
  name: navitia
spec:
  ports:
  - port: 80
    protocol: TCP
    targetPort: 80
  selector:
    app: app-jormun
  type: ClusterIP
""" > ${output}/kubernetes.yml

mkdir -p ${output}/jormun_conf/

cd ${input}
for folder in $(ls -d */); do
    coverage=${folder%%/}
    echo "Configuring ${coverage}"

    if [[ $coverage =~ "_" ]]; then
      echo "I can't handle a coverage name containing a '_' "
      echo "I'll skip coverage ${coverage}"
      continue
    fi

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

      inputType="gtfs"

      rm -f ${output}/${coverage}/gtfs/*
      mkdir -p ${output}/${coverage}/gtfs/
      cp  ${input}/${coverage}/gtfs/* ${output}/${coverage}/gtfs/

      # remove "StopPoint:" prefix on stop point uris'
      sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/stops.txt
      sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/stop_times.txt
      if [[ -e ${input}/${coverage}/gtfs/transfers.txt ]]; then
        sed -i 's/StopPoint://g' ${output}/${coverage}/gtfs/transfers.txt
      fi
    fi

    # copy ntfs data to output if present
    if [[ -e ${input}/${coverage}/ntfs/ ]]; then
      inputType="ntfs"
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

    # copy geopal data to output if present
    if [[ -e ${input}/${coverage}/geopal/ ]]; then
        rm -f ${output}/${coverage}/geopal/*
        mkdir -p ${output}/${coverage}/geopal/
        zip -j -r ${output}/${coverage}/geopal/geopal.zip ${input}/${coverage}/geopal/
    fi

    # copy fusio-geopal data to output if present
    if [[ -e ${input}/${coverage}/fusio-geopal/ ]]; then
        rm -f ${output}/${coverage}/geopal/*
        mkdir -p ${output}/${coverage}/geopal/
        zip -j -r ${output}/${coverage}/geopal/geopal.zip ${input}/${coverage}/fusio-geopal/
    fi

    # binarize
    echo "Launch binarisation"
    rm -f ${output}/${coverage}/data.nav.lz4
    # run python3 /navitia/source/eitri/eitri.py -d ${output}/${coverage}/ -e /usr/bin -o ${output}/${coverage}/data.nav.lz4

    # copy stoptime_loads if present
    if [[ -e ${input}/${coverage}/stoptimes_loads.csv ]]; then
      cp ${input}/${coverage}/stoptimes_loads.csv ${output}/${coverage}/stoptimes_loads.csv
    fi



    # add kraken and loki services to docker for this coverage
    echo """
  loki-${coverage}:
    image: navitia/mc_loki
    environment:
      - RUST_LOG=debug
    volumes:
      - ./${coverage}/:/data

  kraken-${coverage}:
    image: navitia/mc_kraken
    volumes:
      - ./${coverage}:/data
""" >> ${output}/docker-compose.yml



    krakenPort="30000"
    lokiBasicPort="30001"
    lokiLoadsPort="30002"

    # Jormun config files
    # "loki" with basic comparator
    jq -n --arg instance "${coverage}-loki" --arg krakenSocket "tcp://kraken-${coverage}:${krakenPort}" --arg lokiSocket "tcp://loki-${coverage}:${lokiBasicPort}" '{
    key: $instance,
    zmq_socket: $krakenSocket,
    pt_zmq_socket : $lokiSocket
}'  > ${output}/jormun_conf/${coverage}-loki.json

    # Jormun config files
    # old fashion Kraken
    jq -n --arg instance "${coverage}" --arg krakenSocket "tcp://kraken-${coverage}:${krakenPort}" '{
    key: $instance,
    zmq_socket: $krakenSocket
}'  > ${output}/jormun_conf/${coverage}.json

    # kraken config file
    echo "[GENERAL]
instance_name = ${coverage}
database = /data/data.nav.lz4
zmq_socket = tcp://*:${krakenPort}

" > ${output}/${coverage}/kraken.ini

    # Loki config files
    jq -n --arg basicSocket "tcp://*:$lokiBasicPort" \
          --arg loadsSocket "tcp://*:$lokiLoadsPort" \
          --arg inputType "$inputType" \
          --arg inputPath "/data/$inputType/" \
          '{
    input_data_path: $inputPath,
    input_data_type: $inputType,
    loads_data_path: "/data/stoptimes_loads.csv",
    basic_requests_socket: $basicSocket,
    loads_requests_socket: $loadsSocket,
    data_implem: "periodic",
    criteria_implem: "loads"
}' > ${output}/${coverage}/loki_config.json


  # add kraken and loki services to kubernetes for this coverage
    # kraken config file
    echo """
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: deployment-kraken-${coverage}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: app-kraken-${coverage}
  template:
    metadata:
      labels:
        app : app-kraken-${coverage}
    spec:
      containers:
        - image: navitia/mc_kraken
          name: kraken-${coverage}
          volumeMounts:
            - name: volume-navitia
              mountPath: /data
              subPath: ${coverage}
          ports:
            - containerPort: ${krakenPort}
      volumes:
        - name: volume-navitia
          persistentVolumeClaim:
            claimName: volume-claim-navitia
---
apiVersion: v1
kind: Service
metadata:
  name: kraken-${coverage}
spec:
  ports:
  - port: ${krakenPort}
    protocol: TCP
    targetPort: ${krakenPort}
  selector:
    app: app-kraken-${coverage}
  type: ClusterIP
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: deployment-loki-${coverage}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: app-loki-${coverage}
  template:
    metadata:
      labels:
        app : app-loki-${coverage}
    spec:
      containers:
        - image: navitia/mc_loki
          name: loki-${coverage}
          volumeMounts:
            - name: volume-navitia
              mountPath: /data
              subPath: ${coverage}
          ports:
            - containerPort: ${lokiBasicPort}
            - containerPort: ${lokiLoadsPort}
      volumes:
        - name: volume-navitia
          persistentVolumeClaim:
            claimName: volume-claim-navitia
---
apiVersion: v1
kind: Service
metadata:
  name: loki-${coverage}
spec:
  ports:
  - port: ${lokiBasicPort}
    protocol: TCP
    targetPort: ${lokiBasicPort}
    name: basic
  - port: ${lokiLoadsPort}
    protocol: TCP
    targetPort: ${lokiLoadsPort}
    name: loads
  selector:
    app: app-loki-${coverage}
  type: ClusterIP
""" >> ${output}/kubernetes.yml

    echo "${coverage} done"
done


chmod -R 777 ${output}
