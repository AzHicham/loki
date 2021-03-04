docker run -v "$PWD/data":/data -v /var/run/docker.sock:/var/run/docker.sock   mc_navitia/bina 
docker run mc_jormun

PYTHONPATH=..:../../navitiacommon/ JORMUNGANDR_INSTANCES_DIR=~/jormung_conf/ FLASK_APP=jormungandr:app flask run