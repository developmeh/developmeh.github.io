+++
title = "This Week's Crazy Idea"
template = "page.html"
weight = 1
draft = false
date = 2025-06-08
updated = 2025-06-21
[extra]
desc = "Exploring unconventional ideas about OpenTelemetry, debugging tools, WebRTC, and reimagining internet technologies for a more personal and less centralized web experience"
keywords = "OpenTelemetry, tracing, debugging, GDB, WebRTC, self-hosting, decentralized internet, NAT traversal, dynamic DNS, personal web hosting, p2p connections, Krappy Internet, home hosting"
+++

# This Week's Crazy Idea

In all honestly tech is completely boring. Nothing shakes me to my core anymore. Remember Web-Rings? I do, they sucked but it was a time when peoples ideas were well constrained by context. Back in those days you had to get a host, write you own HTML and CSS, accessability concerns of the time aside, things were ugly and simple. It made up for all the complexity of getting a few paragraphs to show up on someone elses screen. We kinda gave all that up for the global town square, just worry about the paragraph and maybe some photos, the content baby. Like a small business there is some charm in the agency to make something fantastic and utterly fail at it. Its about the effort and the intent, that being self-expression, and the barrier to entry was just heavy enough to keep the boring outa the way.

A price we pay for giving everyone a voice is not everyone has something interesting to say. Its that little pain like wanting to run a newsletter for a thousand people but having a dot-matrix printer. There is a little nagging voice in the back of your head saying, I cant listen to that think print a thousand sheets just so someone can throw it out. But it was just that kind of drive that got you to do it, the creative act of getting someone to react to what your wrote.

Thats what this is all about, the creative act. It's not doing things because they are profitable or even relevant, but because they are interesting or fun.

Though in some ways I am talking about giving every internet connected person a voice but one that they control and not one that promotes clout. There is clearly a value to a central platform for discovery, and in some past world that was the responsibility of the search engine. Now I think this is more about append histories instead of sitemaps and some very clever automation for a federation that provides an index of the internet.

## DevLog

## 21 06 2025
### OpenTelemetry and the question of ditching logs

This morning I had this thought that maybe one of the reasons tracing and Open Telemetry are kind of after thoughts in about 99% of the enterprise projects I work on may be the developer tools gap. Consider this, as a developer many of us only experience tracing "In Production" and only through a rather expensive platform. Is there really a place where tracing is the new debugging. See also those same 99% of enterprise projects also moved to structured logging a while back and to me, the structured log is a trace done poorly. That's an opinion of course but its informed by the fact that most of the time I need distributed correlation more than I need information about the state of the request. When I think of selective logging, I find that I am often making the choice of what not to log where with tracing the only thing I am missing is the context.

Anyways, the point isn't to try an convince anyone to go one way or the other, but the utilization would be greater if more of the tools were used during development. Here is where my ultimately crazy idea comes in. Jager and ZipKin are great but I don't really want to run an ELK (Elasticsearch/Logstash/Kibana) stack on my dev machine. Its a lot of extra setup and its a bit fiddly. I like to think of developer tools as just the basics of a production system. It also makes me think of how we just GDB and other debuggers. We execute them are runtime and use them to debug a specific process, often around a test. When I observe myself and other developers we tend to drop a lot of breakpoints on and around the flaw to identify the code flow that leads to the failing condition. I think of step into and step through functionality of GDB and I want a way to also get detailed trace info at the same time.

Guess what, its not just a crazy idea, its kind of a dumb one. Here is what I learned from the experience. Firstly, I tried to write my own OTEL collector in golang. Not so bad, but processing and visualizing all the traces as a waterfall was a little challenging. My work in progress on [Github](https://github.com/ninjapanzer/otel-tracer). So after I learned a whole lot about tracing and Open Telemetry I cam back to the drawing board and though how would this look if it was part of GDB already. The fun fact is that its kinda already there, not in this irrelevant auto instrumented way that I am proposing but in the nature of whats called a "tracepoint". Check it out I put together a sample you can try yourself as long as you have Go installed. [Debug Tracing](https://github.com/developmeh/debug-tracing).

So the short answer, yes there should be something easier than Jaeger and ELK locally to explore OTEL, but if you wanna enhance your own development process. Time to get comfortable with some more of the debugger tools that already have valuabl tracing and frame logging built in.

When you are in a tool like Goland or IntelliJ, you can have it add something more akin to logs at tracepoints so you don't have to stop on those or modify your code. Where GDB is powerful is it works on your binaries but language level tools work on the runtime code.

Expect more about a lightweight OTEL tracer for exploring traces locally too.

## 15 06 2025
### WebRTC and what not to ask AI to do

So to my great surprise I figured that the LLMs would be the right place to funnel my learnings about WebRTC. A technology that has been just outside my vision since I started my career. Why shouldn't I assume that building a trivial implementation with it with LLM support would save me a lot of cognitive overhead, given the long context of such a technology. I was wrong, it seems that as I delve into the underbelly of network topologies away from the chrome of NextJS and CLI tools the bottom falls our of the LLM as well. Its been a consistent thing on my radar that LLMs are only good at the tasks that push products to market but not the work that makes the products work.

Here is an enumeration of things that the LLMs tend to struggle with:

- Maintaining complex conditional states -- when logical nesting is needed it tends to get confused and will cycle back and forth breaking, fixing, and re-breaking sequences of operations
- Understanding anything about internet topology including TLDs, eTLDs, eTLD+1, and private registries -- while working on the [Passkey Origin Validator](https://github.com/developmeh/passkey-origin-validator) I was amazed that when I presented these concepts it generally couldn't maintain coherence about the meaning of those terms even though they are rather central to how domains work.
- Establishing well documented network handshakes -- Something of a combination of the previous two. There is often a kind of ballet that happens establishing standard and p2p network connections. Since its a set of nested conditionals and requires an understanding of how time works, it struggles.
- Dealing with dependency version changes -- My favorite class of failure, if the library changes the name of a package or a constant the LLM will just assume that the library is broken and remove it. What I find the most awkward is since the LLM is interacting with my computer and my project it has access to my dependencies and could search it to try and resolve the change.

On the other hand a few items I think it nails every time:
- CI/CD pipelines -- Every time I need to run tests on a branch or release on a tag. The LLM handles it in one go.
- CLI Frameworks -- Cobra nad Viper, for example an LLM sets up a fantastic set of arguments, config files and considers a lot of the edge cases for comfortable CLI use by humans.
- Sequence Diagrams -- When I wanna learn a new technology finding a "basic" diagram for how it works is rather annoying. Theres always lots of specs to read but all the pictures are build dependent on a use-case. For example this one it built for my exploration [WebRTC](https://github.com/developmeh/webrtc-poc/blob/master/WEBRTC_CONNECTION_DETAILS.md#mermaid-sequence-diagram)

So in the end I got some joy from the LLM with WebRTC but I kinda had to treat it like a slow version of myself that is also blind and doesn't like to do a web search. I had it explain in a doc how it should work for itself and then asked it to make a boiler plate project with lots of debugging messages. It struggled a lot even with this guidance and I am sure I could have done the same work myself and gained a deeper understanding if I hadn't asked it to do the work.

As this is part of the bigger [Krappy-Internet](https://sr.ht/~ninjapanzer/krappy_internet/) project I then used this poc to try and fix its previous failed implementation. But clearly there is a conceptual block for how the LLM deals with network debugging that it couldn't take a working version and use it to fix a broken version. I did learn something in the process but if this was an actual work activity I would have been stressed, instead of just killing time between blog posts on a rainy Sunday.

## 14 06 2025
### WebRTC, NAT Traversals, and American Manufacturing

So my new view of the architecture required to handle something like dynamic home hosting still requires a method for establishing a p2p connection. While this isn't that big of a deal it does require a consistent connection to be publicly accessible somewhere that is not behind a firewall. Which is rather annoying when trying to make this whole thing work on a phone. It is possible to run a webrtc signaling server phones tend to use "Carrier Grade NAT" CGN means there is no port-forwarding so the phone cannot respond to the signaling request to establish a NAT bypass. I think in this case its still possible but I am uncertain how the signaling server will connect the phone to the browser client when its not expecting to make a connection since it might be asleep.

The next pass would be that this isn't really the best solution for the phone. But general processing would be. Since the point of the phones interaction is to allow the owner of the site to have content interactions follow them it might be appropriate to produce a secure append only log and require the sites submission features require an _Always On_ host to handle requests but this is also a good case for a serverless function. While its still on a cloud provider it could also be handled by a DHT. In that case the easy path would be a function which can accept data requests and append them to a signed log on the same site. The phone of course can then poll the log and prompt the user for activity. Since the polling trivial and we don't actually care about _Real Time_ for these interactions its fine.

Probably the reason its a crazy idea in fact is everything about this rolls back a decades worth of nonsense on the internet from realtime streaming connections to dumping things to files and processing them when its convenient. Its more like reading your email, there isn't really a dopamine hit and the only content that grows is those engaged. The final content is text and permanent. The reason for a lot of real-time communications was to give a faithful response to online transactions, but I see that is one of the ways retailers have complicated buying. They want to allocate inventory but if I am selling maguffins from my garage, inventory is really just a nuance. This isn't a solution for the Amazons of the world, its focus is to create a simpler experience for both a business owner or a blogger. I see the time of complicated sites which have sales funnels is more providing the same value it once did. Deep down we wanna find a thing, buy a thing, and know its gunna show up at some point.

__Simpler__, is probably very subjective but I can see a mechanism around this course work in this project that makes this all a daemon.

In some way this has become a diatribe on why we can't build anything in America. Its because we assume that all items need to be produced at a scale to buy at a Lowes. I think consumer expectations for products is they should be complicated but I think we should start looking back to the items we find at thrift stores. The modality should start to wander towards, "I want to make a good X" not so much "I need a new solutions for X". But thats just my opinion in reality.

## 08 06 2025
### Krappy Internet Dynamic Dns and Hosting at Home

I heard recently that the future of the internet is AI. 🤣 ok ok ok... yes if I was investing a bunch of other peoples money in a technology startup that sold AI I would say a lot of crazy things too. I am not so sure the internet is a "thing" anymore that can go away. Its the substrate for communication and while the way we consume the internet may change there will always need to be a source of personal expression. For the age I come from that would have been the blog, the forum, and the comments section. I was there when Twitter started but it wasn't my thing. I am from the days of GeoCities and Anglefire, shared hosting where a hand-full of webpages was enough to give you a voice. All the backgrounds were tessellated poorly, the text was an odd color but the vibes were true. Frequenting final-fantasy fan sites and reading conspiracies about aliens.

I have this wild idea that the answer for the kludge that is "return the means of production to the people!" The forever cry of the decentralized internet, most of us have multiple internet providers and we have computers just burning dead dinosaurs to watch useless noise videos with plenty of capacity to share.

Regarde-moi! What if we just hosted our own content from our own machines in our own houses? What if it didn't really matter when that server was offline? See there are a lot of us and none of us have anything that's interesting to say, which is a kind of magic when you think how much we talk. It's the community not the communication that matters, we need to feel connected, which is exactly the power of the internet.

So here is the project [https://git.sr.ht/~ninjapanzer/krappy-dyndns](https://git.sr.ht/~ninjapanzer/krappy-dyndns)

The assumption is that if you own a domain you likely also own some free hosting, really lame html hosting but a small piece of the internet that is yours as long as you pay for it. Kinda like a house and property taxes... but lets not go down that road. So your ISP gives you a ton of bandwidth so you can watch _Better Call Saul_ on Netflix but whats it doing when you aren't binging? Just idling like car insurance... but lets not go down that road either. Point is theres a lot of spare internet for the 50 people a month that are going to look at your website. That's pretty cool to be honest when you think about the number of people you might interact with on the average Friday at your local coffee shop. So here are the problems we need so solve:

- give your "special content" the impression its from a fixed location for the sake of discoverability
- find a normal way to allow a browser or application to call back from your internet house to your house house without being bungled by your ISP
- make it easy to maintain some services from your laptop or phone
- keep those things kinda working when those devices are offline

Yes, the idea is to host your site from your phone while its in your pocket. Crazy yes, possibly maybe, am I gunna try, yes.

So back to the point, you own some internet property and with the help of some krappy-dyndns we can publish a text file to the "free" hosting thats attached to your domain. This falls under the guise of what we call these days as .well-known. `https://youraddress.com/.well-known/krappy-dyndns-8abe777a` holding a binary stream of IP address histories and encoded with the name of a service. It's just an IP address and while its your IP address its also shared by others so its vaguely you. The daemon service runs on your target device and on an interval figures out what your IP is and then if it changes pushes it to that well-known file.

A user comes along and wants to leave a comment on your site. It makes a call to the comment service you run on your laptop and the client making the request knows what service it wants to interact with finding the correct .well-known and thus collecting an IP address. Next the tricky part, we have to trick your ISP to accept an incoming connection without an outbound call. Thats the whole NAT thing, probably utilizing something like [https://en.wikipedia.org/wiki/Hole_punching_(networking)](https://git.sr.ht/~ninjapanzer/krappy-dyndns). So your laptop will also host this service on your IP and allow for some underlying protocol like WebRTC to allow the initial transaction and boom the comment has been sent. Now, this is an internet that isn't trying to waste your time, so we take the comment and after its moderated we write it once back to our free hosting and if our laptop gets turned off for the night, who cares, people just cant leave a comment but the imporant stuff stays there. I mean they could always just send an email too.

Just one step in this crazy plan complete this week and another piece of the Krappy Internet is available.
