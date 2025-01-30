+++
title = "The Krappy Internet"
template = "page.html"
updated = 2025-01-29
+++

## What if the internet stopped being shit and was instead Krappy?
The Krappy Internet is an attempt to re-envision how we trust data from the internet. This is barely even a hypothesis but in the pursuit of something closer to what the internet once was without bike shedding blockchains and onion routers I am building my own internet, just for me. Others can use it if it ever does anything.

### Components
Krappy Utils (In Progress) -> https://git.sr.ht/~ninjapanzer/krappy
Krappy Content Linker (In Progress) -> https://git.sr.ht/~ninjapanzer/krappy_internet
Krappy Navigator (Planned)

### Krappy Utils
A persistent connection multiplexing TCP protocol server library. Since everything is going to eventually have a binary protocol it makes sense to hoist that from Krappy Kafka and speed up how fast I can spin up a new protocol processor.

- [ ] Figure out how to test connection management is working as expected.

## DevLog

### 29 01 2025
#### Building a TCP server Library
While this project has been in the works for a while its also an avenue for me to learn. The first task was to build a modern high performance TCP server that has a concept of an easy to manage binary protocol. For this I picked CBOR https://cbor.io/ RFC 8949 Concise Binary Object Representation. Its not the fastest and I am looking for a solution that has a zero copy buffer like flat buffers maybe.

The challenge is making sure that connection management happens as we expect. Since the goal is to allow a client to reuse a connection to stream multiple requests its important that the connection be persistent and also go away as soon as we are done using it so it can be recycled for a future client. In the Krappy Kafka project there are cases where this management appears to get out of sync and blocking causes all go routines to be consumed. Where connections should have been released they were not. Now that project uses a lot of competing mutexes that are likely the cause of deadlocks. The next version of that and all future protocol servers will rely on channels.

From here we move to the Content Linker, in something like a WoT (Web of Trust) model we want to allow content registration for trust. While we want to allow anonymous users to contribute whatever they want we also want content to have a machine like identity. The hope is to promote that content linking is how we establish a chain of custody for truth. User provided consensus then helps to build this trust. This means that content from public identities doesn't have to join a web of trust. Its just available and as it gains consensus the trust of that content is improved as authoritative.

A good model would be wikipedia, Content can be copied and modified but its moderation is the responsibility of the whole. While this doesn't mean that mistruth is evicted, it means that it will often be short lived and even hard to find. Burrying is not something you can effectively pay for but the community can dimish the impact of garbage so much it may never be seen. There are going to need to be some algorithms to help address cheating here but this is the resonsibility of the consumer. The content model is just a weighted data store. You look at whatever you want albeit the model will promote some decisions.
