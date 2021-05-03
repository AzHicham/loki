# Loki

Tips and tricks to fluidify your (public) transit !

## Description

This is a work-in-progress implementation of a multicriteria public transit engine.
The goal is to have an engine that computes a set of pareto-optimal journeys for public transit,
but with an engine generic enough to handle any criteria specified by a partial order.

Note that the engine handle only the 'public transit' part of the journey. It assumes 
that the fallbacks (from the actual starting point to the entrance of the public transit network, as well as from the exit of the public transit network to the actual destination) are given as input.

## Repository Architecture

The root of the repository provides the `loki` library, 
which allow to perform public transit requests on a ntfs/gtfs dataset (read with `transit_model`).

The library can be used with :
- the [stop_areas][1] subcrate, where you can provides the origin and destination as command line arguments,
- the [random][6] to perform benchmarks by generating random requests. 
- as a server with the [server][3] subcrate, which process protobuf journey requests (the format is specified by the [navitia-proto][2] repo) sent to a zmq socket, call the `loki` engine, and return protobuf responses on the zmq socket. 

In order to provide a fully-fledged multimodal journey planner, the `server` needs other [Navitia][4] components as well as data.
These components are bundled together in `./dockers`, and some ready to use data is provided in `./data`.
See the [docker readme][5] for usage.

## Development

To be able to compile this project, you'll need to have libraries for zmq and initialize the submodule that brings protobuf description for Navitia.

```shell
git submodule update --init --recursive
sudo apt install libzmq3-dev
```


## Acknowledgments

This contribution is a part of the research and development work of the
IVA Project which aims to enhance traveler information and is carried out
under the leadership of the Technological Research Institute SystemX,
with the partnership and support of the transport organization authority
Ile-De-France Mobilités (IDFM), SNCF, and public funds
under the scope of the French Program "Investissements d’Avenir".

[1]: ./stop_areas/Readme.md
[2]: https://github.com/CanalTP/navitia-proto
[3]: ./server/Readme.md
[4]: https://github.com/CanalTP/navitia
[5]: ./docker/Readme.md
[6]: ./random/Readme.md