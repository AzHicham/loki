#!/bin/bash
set -e

dir="/data"


# tranform gtfs into ntfs
# gtfs2ntfs --input $dir/gtfs --output $dir/ntfs

# binarize
rm -f $dir/data.nav.lz4
python3 ./navitia/source/eitri/eitri.py -d $dir/ntfs -e /usr/bin -o $dir/data.nav.lz4

# let's now write all config files

instance="mon_instance"

krakenSocket="tcp://kraken:30000"
loadsSocket="tcp://laxatips:30001"
classicSocket="tcp://laxatips:30002"


mkdir -p $dir/jormun_conf/
mkdir -p $dir/laxatips_conf/

# Jormun config files
# one for the "kraken" coverage
jq -n --arg instance "${instance}_kraken" --arg krakenSocket "${krakenSocket}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket
}'  > $dir/jormun_conf/$instance.json
# one for "laxatips" with loads criteria
jq -n --arg instance "${instance}_laxatips_loads" --arg krakenSocket "${krakenSocket}" --arg laxatipsSocket "${loadsSocket}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $laxatipsSocket 
}'  > $dir/jormun_conf/${instance}_loads.json
# one for "laxatips" with classic criteria
jq -n --arg instance "${instance}_laxatips_classic" --arg krakenSocket "${krakenSocket}" --arg laxatipsSocket "${classicSocket}" '{ 
    key: $instance, 
    zmq_socket: $krakenSocket, 
    pt_zmq_socket : $laxatipsSocket 
}'  > $dir/jormun_conf/${instance}_classic.json

# kraken config file
echo "[GENERAL]
instance_name = ${instance}_kraken
database = /data/data.nav.lz4
zmq_socket = $krakenSocket 

[BROKER]
host = rabbitmq
port = 5672
username = guest
password = guest
" > $dir/kraken.ini

# Laxatips config files
# one for the coverage with loads criteria
jq -n --arg laxatipsSocket "$loadsSocket" '{
  ntfs_path: "/data/ntfs/",
  loads_data_path: "/data/stoptimes_loads.csv",
  socket: $laxatipsSocket,
  request_type: "loads",
  implem: "loads_periodic"
}' > $dir/laxatips_conf/loads.json
# one for the coverage with classic criteria
jq -n --arg laxatipsSocket "$classicSocket"  '{
  ntfs_path: "/data/ntfs/",
  loads_data_path: "/data/stoptimes_loads.csv",
  socket: $laxatipsSocket,
  request_type: "classic",
  implem: "periodic"
}' > $dir/laxatips_conf/classic.json

chmod -R 777 $dir