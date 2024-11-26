+++
title = "Learn Event Streaming by Recreating Kafka"
template = "page.html"
weight = 1
draft = true
+++

# Learn Event Streaming by Recreating Kafka

> **I don't know if I really like Kafka all that much**

That said its an interesting way for applications to communicate. See I have been imagining a global game of life implementation with distributed realtime events for each cell in the world. While that is a rather far off dream it made sense to tackle one of its problems. In the past I have setup Kafka development envs with [Nix](https://github.com/ninjapanzer/game_of_life_kafka/blob/main/flake.nix) this is all pretty easy. It even goes so far as to try and track how many shells are currently running to track shutting down Kafka when all is said and done. The only reason I do this is because Kafka is written in Java, famous for the addage, _"write once use 80% of the resources everywhere."_ For better or worse that has always stunk for me.

> **Kafka can't be that complicated write?**

Thats probably both true and false. The only way to tell would be to try. So while this might be structured like a tutorial its really a devlog of the failures to interpret the features.

## Goals
To start we want to create a realtime streaming platform that provides at least:
- A binary protocol
- A protocol built on TCP
- TCP connection multiplexing for consumers and producers
- Event Stream and Log Sequence Merge storage
- Be filesystem oriented where possible
- Don't invent everything just whats needed
- GUI testing and debugging tools
- Protocol client for consumers and producers

## Arch Diagram
The architecture is evolutionary since we don't really know what we are building. Its hard to show this sometimes because as it evolves we remove failures and miss out on the actual point of the discovery.

## Protocol Design

<span style="" class="mermaid">
---
title: "Message"
---
packet-beta
  0-31: "Message Size UInt32"
  32-63: "Message Type UInt32"
  64-95: "Message Payload (Variable Length)"
</span>
