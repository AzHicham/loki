Loki
----------

Tips and tricks to fluidify your (public) transit !

.. |Build Status| image:: https://img.shields.io/github/workflow/status/hove-io/loki/Build%20and%20test?logo=github&style=flat-square
    :target: https://github.com/hove-io/loki/actions?query=workflow%3A%22Build+and+test%22
    :alt: Last build

.. |Code Coverage| image:: https://codecov.io/gh/hove-io/loki/branch/master/graph/badge.svg?token=IYF7W6U2NI
    :target: https://codecov.io/gh/hove-io/loki
    :alt: Coverage

+----------------+-----------------+
| Build status   | Code Coverage   |
+----------------+-----------------+
| |Build Status| | |Code Coverage| |
+----------------+-----------------+

Description
=========

This is a work-in-progress implementation of a multicriteria public transit engine.
The goal is to have an engine that computes a set of pareto-optimal journeys for public transit,
but with an engine generic enough to handle any criteria specified by a partial order.

Note that the engine handle only the 'public transit' part of the journey. It assumes
that the fallbacks (from the actual starting point to the entrance of the public transit network, as well as from the exit of the public transit network to the actual destination) are given as input.

Repository Architecture
=========

The root of the repository provides the `loki` library,
which allow to perform public transit requests on a ntfs/gtfs dataset (read with ``transit_model``).

The library can be used with :

* the stop_areas_ subcrate, where you can provides the origin and destination as command line arguments,
* the random_ to perform benchmarks by generating random requests.
* as a server with the server_ subcrate, which process protobuf journey requests (the format is specified by the navitia-proto_ repo) sent to a zmq socket, call the ``loki`` engine, and return protobuf responses on the zmq socket.

In order to provide a fully-fledged multimodal journey planner, the `server` needs other Navitia_ components as well as data.
These components are bundled together in `./dockers`, and some ready to use data is provided in `./data`.
See the docker-readme_ for usage.

Development
=========

To be able to compile this project, you'll need to:

- initialize the submodule that brings protobuf description for Navitia
- have libraries for zmq and PostgreSQL
- have the `lld` linker (faster than the default `ld`) which is not installed on most default Linux distributions

.. code-block::

    git submodule update --init --recursive
    sudo apt install libzmq3-dev libpq-dev lld

Acknowledgments
=========

This contribution is a part of the research and development work of the
IVA Project which aims to enhance traveler information and is carried out
under the leadership of the Technological Research Institute SystemX,
with the partnership and support of the transport organization authority
Ile-De-France Mobilités (IDFM), SNCF, and public funds
under the scope of the French Program "Investissements d’Avenir".

.. _stop_areas: ./stop_areas/Readme.md
.. _navitia-proto: https://github.com/hove-io/navitia-proto
.. _server: ./server/Readme.md
.. _Navitia: https://github.com/hove-io/navitia
.. _docker-readme: ./docker/Readme.md
.. _random: ./random/Readme.md
