
## Dockers

# Build
You can build the dockers by launching 
```bash
./build_docker.sh -o my_github_token
```
from the root directory of this repository, where `my_github_token` is a OAuth token for github.

# Binarize

Put a gtfs dataset in `./docker/data/gtfs`
Then run  :
```bash
docker run -v "$PWD/data":/data -v /var/run/docker.sock:/var/run/docker.sock   mc_navitia/bina 
```

# Launch

In `./docker` run 
```bash
docker-compose -f compose.yml up
```

Then you can send http requests to the jormun server on http://localhost:9191 !
