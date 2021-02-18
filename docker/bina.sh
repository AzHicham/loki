#!/bin/bash

mkdir -p /data/jormun_conf/
instance="mon_instance"
jq -n --arg instance "$instance" ' { key: $instance, zmq_socket: "ipc:///tmp/kraken" }'  #> /data/jormun_conf/$instance.json
jq -n --arg instance "${instance}_loki" ' { key: $instance, zmq_socket: "ipc:///tmp/kraken", pt_zmq_socket : "ipc:///tmp/kraken" }'  #> /data/jormun_conf/$instance.json

echo "{[GENERAL]
instance_name = $instance
database = /data/data.nav.lz4
zmq_socket = ipc:///tmp/kraken" > kraken.ini



python3 ./navitia/source/eitri/eitri.py -d /data -e /usr/bin -o /data/data.nav.lz4
