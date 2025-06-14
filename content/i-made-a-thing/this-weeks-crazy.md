+++
title = "This Week's Crazy Idea"
template = "page.html"
weight = 1
draft = false
date = 2025-06-08
updated = 2025-06-14
+++

# This Week's Crazy Idea

In all honestly tech is completely boring. Nothing shakes me to my core anymore. Remember Web-Rings? I do, they sucked but it was a time when peoples ideas were well constrained by context. Back in those days you had to get a host, write you own HTML and CSS, accessability concerns of the time aside, things were ugly and simple. It made up for all the complexity of getting a few paragraphs to show up on someone elses screen. We kinda gave all that up for the global town square, just worry about the paragraph and maybe some photos, the content baby. Like a small business there is some charm in the agency to make something fantastic and utterly fail at it. Its about the effort and the intent, that being self-expression, and the barrier to entry was just heavy enough to keep the boring outa the way.

A price we pay for giving everyone a voice is not everyone has something interesting to say. Its that little pain like wanting to run a newsletter for a thousand people but having a dot-matrix printer. There is a little nagging voice in the back of your head saying, I cant listen to that think print a thousand sheets just so someone can throw it out. But it was just that kind of drive that got you to do it, the creative act of getting someone to react to what your wrote.

Thats what this is all about, the creative act. It's not doing things because they are profitable or even relevant, but because they are interesting or fun.

## DevLog

### 14 06 2025
#### WebRTC, NAT Traversals, and American Manufacturing

So my new view of the architecture required to handle something like dynamic home hosting still requires a method for establishing a p2p connection. While this isn't that big of a deal it does require a consistent connection to be publicly accessible somewhere that is not behind a firewall. Which is rather annoying when trying to make this whole thing work on a phone. It is possible to run a webrtc signaling server phones tend to use "Carrier Grade NAT" CGN means there is no port-forwarding so the phone cannot respond to the signaling request to establish a NAT bypass. I think in this case its still possible but I am uncertain how the signaling server will connect the phone to the browser client when its not expecting to make a connection since it might be asleep.

The next pass would be that this isn't really the best solution for the phone. But general processing would be. Since the point of the phones interaction is to allow the owner of the site to have content interactions follow them it might be appropriate to produce a secure append only log and require the sites submission features require an _Always On_ host to handle requests but this is also a good case for a serverless function. While its still on a cloud provider it could also be handled by a DHT. In that case the easy path would be a function which can accept data requests and append them to a signed log on the same site. The phone of course can then poll the log and prompt the user for activity. Since the polling trivial and we don't actually care about _Real Time_ for these interactions its fine.

Probably the reason its a crazy idea in fact is everything about this rolls back a decades worth of nonsense on the internet from realtime streaming connections to dumping things to files and processing them when its convenient. Its more like reading your email, there isn't really a dopamine hit and the only content that grows is those engaged. The final content is text and permanent. The reason for a lot of real-time communications was to give a faithful response to online transactions, but I see that is one of the ways retailers have complicated buying. They want to allocate inventory but if I am selling maguffins from my garage, inventory is really just a nuance. This isn't a solution for the Amazons of the world, its focus is to create a simpler experience for both a business owner or a blogger. I see the time of complicated sites which have sales funnels is more providing the same value it once did. Deep down we wanna find a thing, buy a thing, and know its gunna show up at some point.

__Simpler__, is probably very subjective but I can see a mechanism around this course work in this project that makes this all a daemon.

In some way this has become a diatribe on why we can't build anything in America. Its because we assume that all items need to be produced at a scale to buy at a Lowes. I think consumer expectations for products is they should be complicated but I think we should start looking back to the items we find at thrift stores. The modality should start to wander towards, "I want to make a good X" not so much "I need a new solutions for X". But thats just my opinion in reality.

### 08 06 2025
#### Krappy Internet Dynamic Dns and Hosting at Home

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
