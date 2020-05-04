# Laxatips

Tips and tricks to fluidify your (public) transit !

## Description

This is a work-in-progress implementation of a multicriteria public transit engine.
The goal is to have an engine that computes a set of pareto-optimal journeys for public transit,
but with an engine generic enough to handle any criteria specified by a partial order.

Note that the engine handle only the 'public transit' part of the journey. It assumes 
that the fallbacks (from the actual starting point to the entrance of the public transit network, as well as from the exit of the public transit network to the actual destination) are given as input.

## Architecture

The interface expected by the engine is described in `public_transit.rs`.
The actual engine is implemented in `multicriteria_raptor.rs`.
The engine uses Pareto fronts implemented in `pareto_front.rs`, and store the journeys computed
in a Tree implemented in `journeys_tree.rs`.

## Next steps

- implements the `PublicTransit` interface, using ntfs data read with `transit_model` with "classical" criteria (arrival time, number of transfers, walking time)
- test and debug...
- implements other criteria (overcrowding, train frequency, ...)

