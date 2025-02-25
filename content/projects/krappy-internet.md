+++
title = "The Krappy Internet"
template = "page.html"
date = 2025-01-29
updated = 2025-02-24
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

### 24 02 2025
#### Working around the browser
So one of the challenges of making a side-channel connection to the krappy internet is through a proxy. I don't really see the need to try and forklift the world of current browsers. The plan for this is to create an extension that loads a WASM module wrapping a webrtc data channel. This way I can maintain a socket like stream to another client that is not restricted by the rules of the browser. I can then establish a TCP or QUIC connection to the content tree.

The long road here is probably going to end up being the short one in reality. Browsers are quite irritating and intrusive. I think about how ToR works and how its challenging to link around to things on it. Some of that is due to the impermenance of those servers and the lack of an index. Something like this could act as a generalized bridge between those and other platforms. In the same way that gemini capsules and gopher sites will deploy an http proxy. This proxy is local to the machine so creators can pick any protocol for their site and they could be linked together. I rather like the idea of going to the wallstreet journal and having a tor link to a gemini capsule with the pages content behind the paywall.

It will also be much harder to destroy content as any page that changes can be relinked to something like the internet archive. The control side of this is important, and I wonder if users should opt into other users links. So the defacto nature is we provide our own content and only we can see it, there would need to be some opt in model. I keep seeing it as if the world was one big logseq where content from various location is joined without ownership of any of the sources. Even if it isn't useful its rather cool to think about annotating the internet and building a webring around content that can have a deployed algo track updates.

Dreaming dreams.

For now I am planning on building a PoC from https://github.com/pion/webrtc which will then be compile to WASM and connected to a proxy server.

### 11 02 2025
#### An Ideal World
I see the internet as a great library archive, while I haven't done the math, I expect the rate at which we create material is roughly at the same rate we improve storage density. At least I can account for that in my own life.

So here is a random vision for the internet. I pay for connection to the network. In deference to the world I live in today, that used to mean something a little different in my youth. Something that drives me to view myself a more of a producer/consumer than just a consumer. I am sure I am not alone.

We pay a provider and I get some simple addressable hardware from them, now I get a public IP address but moreover a dynamic DNS built into my hardware. My provider acts a kind of lookup service which allows me to host applications within my infrastructure and make them available to the greater internet. When I share an image, I share it from my network. My provider also acts as a cache so allow my devices and services to be offline without interruption.

It's not an X or Y kind of situation, personally hosted lives alongside the giants. Services like Vercel or Hetzner still exist for hosting. But when I share text to comment on Bluesky I own that text and it is hosted on my device and cached by Bluesky. When I revoke access to my post, its not gone, but its removed from the cache in the same way we handle DNS propagation. It would be a wild and noisy place and the problem to solve is how to find the things you wanna read. The ecosystem for applications changes as well. Everything is a server, I mean it already is except you don't know what its serving and to who...

An idealistic view of a future state that still requires a lot of work.

### 06 02 2025
#### Getting over the Browser
So recently I came to this understanding of the nature of the Modern OS, which includes the web browser. So there are really two ways to go. Create a new browser using an open source project or build a side-channel daemon.

I rather like the daemon concept because getting something integrated and deployed into a bespoke browser build is going to be an unlikely way to get someone to use something.

### 29 01 2025
#### Building a TCP server Library
While this project has been in the works for a while its also an avenue for me to learn. The first task was to build a modern high performance TCP server that has a concept of an easy to manage binary protocol. For this I picked CBOR https://cbor.io/ RFC 8949 Concise Binary Object Representation. Its not the fastest and I am looking for a solution that has a zero copy buffer like flat buffers maybe.

The challenge is making sure that connection management happens as we expect. Since the goal is to allow a client to reuse a connection to stream multiple requests its important that the connection be persistent and also go away as soon as we are done using it so it can be recycled for a future client. In the Krappy Kafka project there are cases where this management appears to get out of sync and blocking causes all go routines to be consumed. Where connections should have been released they were not. Now that project uses a lot of competing mutexes that are likely the cause of deadlocks. The next version of that and all future protocol servers will rely on channels.

From here we move to the Content Linker, in something like a WoT (Web of Trust) model we want to allow content registration for trust. While we want to allow anonymous users to contribute whatever they want we also want content to have a machine like identity. The hope is to promote that content linking is how we establish a chain of custody for truth. User provided consensus then helps to build this trust. This means that content from public identities doesn't have to join a web of trust. Its just available and as it gains consensus the trust of that content is improved as authoritative.

A good model would be wikipedia, Content can be copied and modified but its moderation is the responsibility of the whole. While this doesn't mean that mistruth is evicted, it means that it will often be short lived and even hard to find. Burrying is not something you can effectively pay for but the community can dimish the impact of garbage so much it may never be seen. There are going to need to be some algorithms to help address cheating here but this is the resonsibility of the consumer. The content model is just a weighted data store. You look at whatever you want albeit the model will promote some decisions.
