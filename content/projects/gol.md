+++
title = "Distributed Game of Life"
template = "page.html"
updated = 2024-09-10
+++

## GoL
I have always found simulations exciting. While the Game of Life is a shallow simulation it is fun how fast you can stand it up. In the old days I would always standup a language and create a Rock Paper Scissors game to prove some minor competency. Now its GoL, I like having to build an animation or a statistics engine. What has gotten to me these days is the scale of GoL and then injecting new rules.

### Distributed
So one of the things about distribution I am excited about is the noise from eventual consistency. In a traditional GoL we have what I refer to as the __World__ nothing more than a matrix of state that is roughly binary, alive or dead.

The world is prepopulated with a seed, some intentional or random spattering of alive to get the whole thing started.

Skipping the rules now we extend each cell in our matrix from a binary to a stateful object. Maybe they have names now like "bert" and "harry". They can have progeny and a history.

### Time series and geneology
My first thought was I could track this history using a timeseries db, and even attempted to build on in ERLang. But then I realized I could probably make that a little more interesting if I did it with something distributed like NATS or Kafka.

## Phase 1
So the first phase here is to introduce only a distributed __World__ that can be queried from a compacted topic. Creating some form of client SDK to observe the world graph

## Phase 2
Unbound the graph, focusing only on neighborhoods and seeing if I can just query within a contiguous window of the simulation so I could have different views of the same simulation running at the same time.

## Phase 3
Heredity, try and see if I can trace the lineage of a cell through this process of events and a graph datastore to add new rules to the game as a cells heredity expands.
