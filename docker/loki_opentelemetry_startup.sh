#!/bin/bash

# Adapted from https://docs.docker.com/config/containers/multi-service_container/

# Start opentelemetry exporter
/usr/bin/otelcol-contrib --config=/etc/otelcol/config.yaml &

# Start loki
/usr/local/bin/loki_server &

# Wait for any process to exit
wait -n

# Exit with status of process that exited first
exit $?
