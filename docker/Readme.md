
## Dockers
Provides dockers to run a fully featured navitia, where laxatips_server 
will solve the "public transport" part of the request, instead of kraken.


# Build
You can build the dockers by launching 
```bash
./build_docker.sh -o my_github_token
```
from the root directory of this repository, where `my_github_token` is a OAuth token for github.

# Binarize

Put a gtfs datasets in `./docker/data/` with one folder per instance :
docker/
Then from `.docker` run  :
```bash
docker run -v "$PWD":/storage -v /var/run/docker.sock:/var/run/docker.sock   mc_navitia/bina 
```

This will create a folder `./docker/mc_navitia` containing everything needed to launch navitia.

# Launch

In `./docker/mc_navitia` run 
```bash
docker-compose -f compose.yml up
```

Then you can send http requests to the jormun server on http://localhost:9191 !
