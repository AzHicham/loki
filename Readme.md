# Laxatips

Tips and tricks to fluidify your (public) transit !

## Description

This is a work-in-progress implementation of a multicriteria public transit engine.
The goal is to have an engine that computes a set of pareto-optimal journeys for public transit,
but with an engine generic enough to handle any criteria specified by a partial order.

Note that the engine handle only the 'public transit' part of the journey. It assumes 
that the fallbacks (from the actual starting point to the entrance of the public transit network, as well as from the exit of the public transit network to the actual destination) are given as input.

## Repository Architecture

The root of the repository provides the `laxatips` library, 
which allow to perform public transit requests on a ntfs dataset (read with `transit_model`).

The library can be used as :
- command line interface with the [cli][1] subcrate, where you can provides the origin and destination as command line arguments, or perform benchmarks by generating random requests
- as a server with the [server][3] subcrate, which process protobuf journey requests (the format is specified by the [navitia-proto][2] repo) send to a zmq socket, call the `laxatips` engine, and returns the protobuf response on the zmq socket.

## Laxatips engine architecture

The interface expected by the engine is described in `public_transit.rs`.
The actual engine is implemented in `multicriteria_raptor.rs`.
The engine uses Pareto fronts implemented in `pareto_front.rs`, and store the journeys computed
in a Tree implemented in `journeys_tree.rs`.

## Next steps

- implements the `PublicTransit` interface, using ntfs data read with `transit_model` with "classical" criteria (arrival time, number of transfers, walking time)
- test and debug...
- implements other criteria (overcrowding, train frequency, ...)

[1]: ./cli/Readme.md
[2]: https://github.com/CanalTP/navitia-proto
[3]: ./server/Readme.md