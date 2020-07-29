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
- as a server with the [server][3] subcrate, which process protobuf journey requests (the format is specified by the [navitia-proto][2] repo) sent to a zmq socket, call the `laxatips` engine, and return protobuf responses on the zmq socket.

## Laxatips library Architecture

### Overview

The laxatips library resolve around the [`PublicTransit`](./src/public_transit.rs) trait, an abstract interface 
for a public transit request.
The engine can solve any request that implements this `PublicTransit` trait.


The goal is to implements `PublicTransit` for each kind of request (forward/backward, different criteria, forbidden lines/stops)
and have the engine (same code) for every one of them.
For now there is only one implementation : forward request with a criteria on arrival time and walking time, implemented in [depart_after.rs](./src/request/depart_after.rs).

In order to construct a `Request` structure implements the `PublicTransit` trait (such as the one in [depart_after.rs](./src/request/depart_after.rs)), one needs two inputs :
 - data provided by the traveller : the allowed departure and arrival stop points, along with their fallback durations
 - data describing the public transit network

Handling the data provided by the traveller is the job of the two subcrates [cli][1] and [server][3] :
they process the input (from the command line, or from a protobuf request) and feed them into the `Request`.

Handling data describing the public transit network is the job of the [`TransitData`](./src/transit_data/data.rs) struct.
A `TransitData` provides queries on the public network structure (stops, transfers, vehicles journeys, etc.)
in a form that ease the implementation of the `PublicTransit` trait. 

For example, `TransitData` groups together the vehicle journeys that deserves the same stops and that does not takeover (a takeover is when a vehicle A leaves earlier than vehicle B, but A arrives later than B). 
Each group is called a `Mission`, which maps to the `Mission` type needed to implements `PublicTransit`.
More importantly, `TransitData` can compute the earliest vehicle of a `Mission` that can be boarded at a given time. 


A `TransitData` is constructed from a `transit_model::Model`, which itself can be built from an ntfs dataset (or any other kind of dataset handled by the [transit_model](https://crates.io/crates/transit_model) crate).

Note that a `TransitData` does not keep a pointer to the `transit_model::Model` used to built it. 
It may change in the future.

So the overall "setup step" is :
- read a dataset to build a `transit_model::Model`
- build a `TransitData` from the `transit_model::Model`
- obtain traveller input
- using the traveller input and the `TransitData`, build a `Request` that implements `PublicTransit` 
- ask the engine to solve the `Request`.







### Directory structures

- src/request for implementations of the `PublicTransit` trait
- src/engine for the algorithm that solve a `PublicTransit` request

Interface between the engine and the request/response
so as to have an engine able to handle different kind of request (forward/backward, different criteria, forbidden lines/stops)

Data comes from ntfs (transit_model) and request can come from different places (cli, server)

ntfs -> transit_model -> transit_data |-> public_transit implem -> engine -> public_transit::Journey  | -> reponse::Journey  | ->  
                              request |                                      public_transit implem    |

transit_data -> 


### Public Transit Interface

### Implementation(s) of the Public Transit Interface

### Engine

The interface expected by the engine is described in `public_transit.rs`.
The actual engine is implemented in `multicriteria_raptor.rs`.
The engine uses Pareto fronts implemented in `pareto_front.rs`, and store the journeys computed
in a Tree implemented in `journeys_tree.rs`.

## Next steps


- an engine struct need to be spawn for each kind of request. Can we reuse the same struct for multiple kinds ?
- 

- implements the `PublicTransit` interface, using ntfs data read with `transit_model` with "classical" criteria (arrival time, number of transfers, walking time)
- test and debug...
- implements other criteria (overcrowding, train frequency, ...)

[1]: ./cli/Readme.md
[2]: https://github.com/CanalTP/navitia-proto
[3]: ./server/Readme.md