
## Dockers
Provides dockers to run a fully featured navitia, where loki_server 
will solve the "public transport" part of the request, instead of kraken.


# Build
You can build the dockers by launching 
```bash
./build_docker.sh -o my_github_token
```
from the root directory of this repository, where `my_github_token` is a OAuth token for github.

# Binarize

Put gtfs datasets in `./data/` with one folder per instance.
You can also add osm data.
In the following example, we have two datasets (auvergne and idfm), with osm data provided only for idfm.
├── auvergne
│   ├── gtfs
│   │   ├── agency.txt
│   │   ├── calendar.txt
│   │   ├── routes.txt
│   │   ├── stop_extensions.txt
│   │   ├── stops.txt
│   │   ├── stop_times.txt
│   │   └── trips.txt
│   └── stoptimes_loads.csv
└── idfm
    ├── gtfs
    │   ├── agency.txt
    │   ├── calendar.txt
    │   ├── routes.txt
    │   ├── stop_extensions.txt
    │   ├── stops.txt
    │   ├── stop_times.txt
    │   └── trips.txt
    ├── osm
    │   └── paris.osm.pbf
    └── stoptimes_loads.csv

Then, from the root directory of this repository, run :

```bash
docker run -v "$PWD":/storage -v /var/run/docker.sock:/var/run/docker.sock   mc_navitia/bina 
```

This will create a folder `./mc_navitia` containing everything needed to launch navitia.

# Launch

In `./mc_navitia` run 
```bash
docker-compose up
```

Then you can send http requests to the jormun server on http://localhost:9191 !
