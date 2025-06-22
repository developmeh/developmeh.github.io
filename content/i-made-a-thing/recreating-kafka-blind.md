+++
title = "Learn Event Streaming by Recreating Kafka"
template = "page.html"
weight = 1
draft = false
date = 2024-09-10
updated = 2025-06-21
[extra]
desc = "A developer's journey recreating Kafka from scratch to understand event streaming, including implementation details, challenges, and lessons learned"
keywords = "Kafka, event streaming, Go, Golang, distributed systems, message broker, log-structured merge, pebble DB, TCP protocol, consumer groups"
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
<svg xmlns="http://www.w3.org/2000/svg" style="cursor:pointer;max-width:100%;max-height:231px;" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1" width="481px" viewBox="-0.5 -0.5 481 231" content="&lt;mxfile host=&quot;app.diagrams.net&quot; agent=&quot;Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0&quot; version=&quot;26.0.3&quot;&gt;&#xA;  &lt;diagram name=&quot;Page-1&quot; id=&quot;87HI9CB669h_auib6GwQ&quot;&gt;&#xA;    &lt;mxGraphModel dx=&quot;2524&quot; dy=&quot;1296&quot; grid=&quot;1&quot; gridSize=&quot;10&quot; guides=&quot;1&quot; tooltips=&quot;1&quot; connect=&quot;1&quot; arrows=&quot;1&quot; fold=&quot;1&quot; page=&quot;1&quot; pageScale=&quot;1&quot; pageWidth=&quot;850&quot; pageHeight=&quot;1100&quot; math=&quot;0&quot; shadow=&quot;0&quot;&gt;&#xA;      &lt;root&gt;&#xA;        &lt;mxCell id=&quot;0&quot; /&gt;&#xA;        &lt;mxCell id=&quot;1&quot; parent=&quot;0&quot; /&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-1&quot; value=&quot;Krappy Server&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;verticalAlign=top;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;280&quot; y=&quot;120&quot; width=&quot;280&quot; height=&quot;230&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-7&quot; style=&quot;edgeStyle=orthogonalEdgeStyle;rounded=0;orthogonalLoop=1;jettySize=auto;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; source=&quot;EgLxcdtnQqiLWyNZ09j0-2&quot; target=&quot;EgLxcdtnQqiLWyNZ09j0-1&quot; edge=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry relative=&quot;1&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-2&quot; value=&quot;Producer Client&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;80&quot; y=&quot;120&quot; width=&quot;120&quot; height=&quot;60&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-4&quot; value=&quot;LSM Store&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;290&quot; y=&quot;190&quot; width=&quot;120&quot; height=&quot;60&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-5&quot; value=&quot;Stream Store&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;430&quot; y=&quot;190&quot; width=&quot;120&quot; height=&quot;60&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-6&quot; value=&quot;Consumer Group Props&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;290&quot; y=&quot;270&quot; width=&quot;120&quot; height=&quot;60&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-9&quot; style=&quot;edgeStyle=orthogonalEdgeStyle;rounded=0;orthogonalLoop=1;jettySize=auto;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; source=&quot;EgLxcdtnQqiLWyNZ09j0-8&quot; target=&quot;EgLxcdtnQqiLWyNZ09j0-1&quot; edge=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry relative=&quot;1&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;        &lt;mxCell id=&quot;EgLxcdtnQqiLWyNZ09j0-8&quot; value=&quot;Consumer Client&quot; style=&quot;rounded=0;whiteSpace=wrap;html=1;fillColor=#eeeeee;strokeColor=#36393d;&quot; parent=&quot;1&quot; vertex=&quot;1&quot;&gt;&#xA;          &lt;mxGeometry x=&quot;80&quot; y=&quot;260&quot; width=&quot;120&quot; height=&quot;60&quot; as=&quot;geometry&quot; /&gt;&#xA;        &lt;/mxCell&gt;&#xA;      &lt;/root&gt;&#xA;    &lt;/mxGraphModel&gt;&#xA;  &lt;/diagram&gt;&#xA;&lt;/mxfile&gt;&#xA;" onclick="(function(svg){var src=window.event.target||window.event.srcElement;while (src!=null&amp;&amp;src.nodeName.toLowerCase()!='a'){src=src.parentNode;}if(src==null){if(svg.wnd!=null&amp;&amp;!svg.wnd.closed){svg.wnd.focus();}else{var r=function(evt){if(evt.data=='ready'&amp;&amp;evt.source==svg.wnd){svg.wnd.postMessage(decodeURIComponent(svg.getAttribute('content')),'*');window.removeEventListener('message',r);}};window.addEventListener('message',r);svg.wnd=window.open('https://viewer.diagrams.net/?client=1&amp;page=0&amp;edit=_blank');}}})(this);"><defs><style type="text/css">@import url(https://fonts.googleapis.com/css2?family=Architects+Daughter:wght@400;500);&#xa;</style></defs><g><g><rect x="200" y="0" width="280" height="230" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe flex-start; justify-content: unsafe center; width: 278px; height: 1px; padding-top: 7px; margin-left: 201px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">Krappy Server</div></div></div></foreignObject><text x="340" y="19" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">Krappy Server</text></switch></g></g><g><path d="M 120 30 L 160 30 L 160 115 L 193.63 115" fill="none" stroke="#36393d" style="stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke-miterlimit="10" pointer-events="stroke"/><path d="M 198.88 115 L 191.88 118.5 L 193.63 115 L 191.88 111.5 Z" fill="#36393d" style="fill: light-dark(rgb(54, 57, 61), rgb(186, 189, 192)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" stroke-miterlimit="10" pointer-events="all"/></g><g><rect x="0" y="0" width="120" height="60" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe center; justify-content: unsafe center; width: 118px; height: 1px; padding-top: 30px; margin-left: 1px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">Producer Client</div></div></div></foreignObject><text x="60" y="34" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">Producer Client</text></switch></g></g><g><rect x="210" y="70" width="120" height="60" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe center; justify-content: unsafe center; width: 118px; height: 1px; padding-top: 100px; margin-left: 211px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">LSM Store</div></div></div></foreignObject><text x="270" y="104" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">LSM Store</text></switch></g></g><g><rect x="350" y="70" width="120" height="60" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe center; justify-content: unsafe center; width: 118px; height: 1px; padding-top: 100px; margin-left: 351px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">Stream Store</div></div></div></foreignObject><text x="410" y="104" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">Stream Store</text></switch></g></g><g><rect x="210" y="150" width="120" height="60" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe center; justify-content: unsafe center; width: 118px; height: 1px; padding-top: 180px; margin-left: 211px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">Consumer Group Props</div></div></div></foreignObject><text x="270" y="184" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">Consumer Group Props</text></switch></g></g><g><path d="M 120 170 L 160 170 L 160 115 L 193.63 115" fill="none" stroke="#36393d" style="stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke-miterlimit="10" pointer-events="stroke"/><path d="M 198.88 115 L 191.88 118.5 L 193.63 115 L 191.88 111.5 Z" fill="#36393d" style="fill: light-dark(rgb(54, 57, 61), rgb(186, 189, 192)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" stroke-miterlimit="10" pointer-events="all"/></g><g><rect x="0" y="140" width="120" height="60" fill="#eeeeee" style="fill: light-dark(rgb(238, 238, 238), rgb(32, 32, 32)); stroke: light-dark(rgb(54, 57, 61), rgb(186, 189, 192));" stroke="#36393d" pointer-events="all"/></g><g><g transform="translate(-0.5 -0.5)"><switch><foreignObject style="overflow: visible; text-align: left;" pointer-events="none" width="100%" height="100%" requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"><div xmlns="http://www.w3.org/1999/xhtml" style="display: flex; align-items: unsafe center; justify-content: unsafe center; width: 118px; height: 1px; padding-top: 170px; margin-left: 1px;"><div style="box-sizing: border-box; font-size: 0; text-align: center; color: #000000; "><div style="display: inline-block; font-size: 12px; font-family: &quot;Helvetica&quot;; color: light-dark(#000000, #ffffff); line-height: 1.2; pointer-events: all; white-space: normal; word-wrap: normal; ">Consumer Client</div></div></div></foreignObject><text x="60" y="174" fill="light-dark(#000000, #ffffff)" font-family="&quot;Helvetica&quot;" font-size="12px" text-anchor="middle">Consumer Client</text></switch></g></g></g><switch><g requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"/><a transform="translate(0,-5)" xlink:href="https://www.drawio.com/doc/faq/svg-export-text-problems" target="_blank"><text text-anchor="middle" font-size="10px" x="50%" y="100%">Text is not SVG - cannot display</text></a></switch></svg>


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

## DevLog

## 25 12 2024
Having now deployed this to k8s through k0s on a local but remote server I have noticed there are throughput issues. While, on the same machine it is possible for a high velocity producer to continuously send events at raw go runtime speed while the server consumes them. But the server is a little more limited and we are finding that either due to disk access speed introduced by containerd or that systems slower architecture we can easily exceed the available threads and crash the app.

I have a couple of ideas of how to handle this:
1. Right now I allocate and write to both a log file and pebbled db on each message received. I might be better batching writes and feeding them into pebble through a channel.
2. The condition seems to be limited to producer events and possibly there is a bug related to how connections are closed. Possibly, they are not closing immediately on client close and are waiting 5 seconds, effectively backlogging.

## 22 12 2024
There has been some work setting up k0s and learning the toolchains involved. I also integrated a build pipeline using Nix. Allows this project to now produce a 40mb image that is easy to deploy to my local k0s. Intentionally, this k8s instance is on a remote machine so I have at least a small non-localhost network effect when testing. I refactored how event handlers are declared under a specific interface for handlers.

```go
func (h *Handlers) ExecuteHandler(name string, ctx context.Context, contract interface{}) (context.Context, error)
func (h *Handlers) ExecuteWithWriteHandler(name string, ctx context.Context, contract interface{}, w io.Writer) (context.Context, error)
```

while I dont know the normality of creating functional interfaces in Golang, this felt more natural than constructing a type because I had some regular variance in types to controll write access.

That said a message handler did become a type
```go
type Handlers struct {
  s               *Server
  messageHandlers map[string]func(ctx context.Context, contract interface{}, writer io.Writer) (context.Context, error)
}
```

Handler registration is then

```go
handlers := NewHandlers(s)
handlers.RegisterHandler(ConsumerRegistrationHandler, consumerRegistration)
handlers.RegisterHandler(ProducerRegistrationHandler, producerRegistration)
handlers.RegisterHandler(PollHandler, pollHandler)
handlers.RegisterHandler(MessageHandler, messageHandler)
```

And execution looks like

```go
case ConsumerRegistration:
  if ctx, err = h.ExecuteHandler(ConsumerRegistrationHandler, ctx, m); err != nil {
    slog.Error("Error registering consumer", "Error", err)
    cancel()
    break
  }
  ctx = context.WithValue(ctx, "ConsumerGroup", m.ConsumerName)
```

The variance the aforementioned interface provided is related to if the handler will have access to the connection so it can write messages back to the client. I currently only have a single usecase which involves polling. Since this is based on a TCP connection we can infer ACK and appropriately handle those errors on the connection.

At this point I decided that a 5s context timeout might not account for long running connection and on each message publish we extend the timeout. Generally, my opinion is that if you are actively sending we should keep you alive and allow timely termination if you pause. One concern is that each time context timeout is extended we deepend the context object. I assume this causes it to increase in size. I need to do some research to assure if a connection was kept alive for days it wouldn't prove a memory leak.

## 05 11 2024
Consumer groups

Having implemented a polling mechanism it came to mind that I might have multiple concurrent consumers polling for messages. So I need to maintain a shared offset of the all the consumers registered in a consumer group. So I modified my consumer contract.

```go
type ConsumerRegistration struct {
  ApiContract
  TopicName    string `codec:"topic,string"`
  ConsumerName string `codec:"consumer,string"`
  Offset       uint64 `codec:"offset,uint64"`
}
```

We now allow the consumer to name themselves and this allows us to allocate a new handle for each client reading allowing two clients to read from the same file at different offsets.

```go
func declareConsumer(consumerName string, store *EventStore) (string, error) {
  i := 0
  for {
    var name = consumerName + "_" + strconv.Itoa(i)
    if exists := store.Get(name); exists == nil {
      return name, nil
    } else {
      i++
    }
    if i > 1000 {
      return "", errors.New("Too many consumers")
    }
  }
}
```

While a little hacky and providing an arbitrary limit of 1000 consumers per group per server but we generate a sequential consumername for our event store. This will find the first open gap in the list of 0-1000. I have wondered if I can have a range like coroutine that retains the global sequence but I wanted to ensure the list was not exhausted after 1000 but that only 1000 could exist concurrently.

## 11 09 2024
Addressing handshake I decided with some trepidation to use the go ctx object to hold state while a handler is looping. The sequence is a little something like this

<span style="" class="mermaid">
sequenceDiagram
    Client->>+Server: Open TCP Conn
    Server-->Client: OK
    loop connection
    alt Communicates
    Client->>Server: Register Producer
    Server->>Client: ACK
    Client->>Server: Publish Event
    Server->>Client: ACK
    else
    Client->>Server: Disconnect
    end
    end
    Server->>-Server: CLOSE CONNECTION
</span>

During thisconnection loop we retain a context stack with a timeout of 5 seconds. So each time we connect to the krappy server we have to establish why we are there and that goroutine then waits until there is more data. Each message is handled by the same goroutine. The choice of using context was intentional but I could have also stored that data outside the context in the closure formed by the goroutine.

The same process happens with the consumer registration. I did want the connection to be a reusable as possible though so once producer is registered that connection could be reused to register a different producer. I don't know if there is a usecase for that but without more reasons to want to isolate a context I followed this process.

## 10 09 2024
The first real learning here was about how kafka deals with compacted topics. So this project is blind of the formal implementation so I looked for an algorithm that was designed to collapse a stream of events to its final value. I looked at a number of tree like patterns that would allow me to collect all the events out of sync and then be able to refer to only the latest but I stopped with the LSM (Log-Structued Merge) [Wikipedia](https://en.wikipedia.org/wiki/Log-structured_merge-tree) which I discovered was similar to the rocksdb implementation that kafka uses. I selected pebbleDB as it was based on the original LevelDB implementation I had used in a previous Erlang project. So turns out getting a compacted topic is pretty easy and as long as I can guarantee the write of the produced message I can guarantee that I can have a top value.

The two reasons I selected go for this project was to give me a strong toolchain for concurrency and speed. Golang has great libs for handling network connections and building servers but that still left me to understand how best to build something that could handle massive throughput. I will set my benchmarks running on k8s on an x86-64 linux env.

But this project has a lot to do with managing concurrent resources and mutex which golang does a pretty good job of too.

So the first release includes full stream retention and a compacted topic using leveldb in memory.

I created a couple of e2e tools that allow me to hammer the server both as a producer and consumer but understanding the lifecycle of a connection is my biggest challenge.

Whats ugly about this project is creating a API that requires multiple steps to complete an initial handshake.
