#!/bin/bash
set -e

dir="/data"


# tranform gtfs into ntfs
rm -f $dir/ntfs/*
mkdir -p $dir/ntfs
gtfs2ntfs --input $dir/gtfs --output $dir/ntfs

# binarize
rm -f $dir/data.nav.lz4
python3 ./navitia/source/eitri/eitri.py -d $dir/ntfs -e /usr/bin -o $dir/data.nav.lz4

# let's now write all config files

instance="mon_instance"

krakenPort="30000"
laxatipsBasicPort="30001"
laxatipsLoadsPort="30002"


mkdir -p $dir/jormun_conf/
mkdir -p $dir/laxatips_conf/

# Jormun config files
# one for the "kraken" coverage
jq -n --arg instance "${instance}_kraken" --arg krakenSocket "tcp://kraken:${krakenPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket
}'  > $dir/jormun_conf/$instance.json
# one for "laxatips" with loads comparator
jq -n --arg instance "${instance}_laxatips_loads" --arg krakenSocket "tcp://kraken:${krakenPort}" --arg laxatipsSocket "tcp://laxatips:${laxatipsLoadsPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $laxatipsSocket 
}'  > $dir/jormun_conf/${instance}_loads.json
# one for "laxatips" with basic comparator
jq -n --arg instance "${instance}_laxatips_basic" --arg krakenSocket "tcp://kraken:${krakenPort}" --arg laxatipsSocket "tcp://laxatips:${laxatipsBasicPort}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $laxatipsSocket 
}'  > $dir/jormun_conf/${instance}_classic.json

# kraken config file
echo "[GENERAL]
instance_name = ${instance}_kraken
database = /data/data.nav.lz4
zmq_socket = tcp://*:${krakenPort}

[BROKER]
host = rabbitmq
port = 5672
username = guest
password = guest
" > $dir/kraken.ini

# Laxatips config files
# one for the coverage with loads criteria
jq -n --arg basicSocket "tcp://*:$laxatipsBasicPort" --arg loadsSocket "tcp://*:$laxatipsLoadsPort" '{
  ntfs_path: "/data/ntfs/",
  loads_data_path: "/data/stoptimes_loads.csv",
  basic_requests_socket: $basicSocket,
  loads_requests_socket: $loadsSocket,
  data_implem: "loads_periodic",
  criteria_implem: "loads"
}' > $dir/laxatips_conf/config.json


chmod -R 777 $dir