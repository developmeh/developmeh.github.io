+++
title = "This Week's Crazy Idea"
template = "page.html"
weight = 1
draft = false
date = 2025-06-08
updated = 2025-06-08
+++

# This Week's Crazy Idea

In all honestly tech is completely boring. Nothing shakes me to my core anymore. Remember Web-Rings? I do, they sucked but it was a time when peoples ideas were well constrained by context. Back in those days you had to get a host, write you own HTML and CSS, accessability concerns of the time aside, things were ugly and simple. It made up for all the complexity of getting a few paragraphs to show up on someone elses screen. We kinda gave all that up for the global town square, just worry about the paragraph and maybe some photos, the content baby. Like a small business there is some charm in the agency to make something fantastic and utterly fail at it. Its about the effort and the intent, that being self-expression, and the barrier to entry was just heavy enough to keep the boring outa the way.

A price we pay for giving everyone a voice is not everyone has something interesting to say. Its that little pain like wanting to run a newsletter for a thousand people but having a dot-matrix printer. There is a little nagging voice in the back of your head saying, I cant listen to that think print a thousand sheets just so someone can throw it out. But it was just that kind of drive that got you to do it, the creative act of getting someone to react to what your wrote.

Thats what this is all about, the creative act. It's not doing things because they are profitable or even relevant, but because they are interesting or fun.

## DevLog

### 08 06 2025
#### Krappy Internet Dynamic Dns and Hosting at Home

I heard recently that the future of the internet is AI. 🤣 ok ok ok... yes if I was investing a bunch of other peoples money in a technology startup that sold AI I would say a lot of crazy things too. I am not so sure the internet is a "thing" anymore that can go away. Its the substrate for communication and while the way we consume the internet may change there will always need to be a source of personal expression. For the age I come from that would have been the blog, the forum, and the comments section. I was there when Twitter started but it wasn't my thing. I am from the days of GeoCities and Anglefire, shared hosting where a hand-full of webpages was enough to give you a voice. All the backgrounds were tessellated poorly, the text was an odd color but the vibes were true. Frequenting final-fantasy fan sites and reading conspiracies about aliens.

I have this wild idea that the answer for the kludge that is "return the means of production to the people!" The forever cry of the decentralized internet, most of us have multiple internet providers and we have computers just burning dead dinosaurs to watch useless noise videos with plenty of capacity to share.

Regarde-moi! What if we just hosted our own content from our own machines in our own houses? What if it didn't really matter when that server was offline? See there are a lot of us and none of us have anything that's interesting to say, which is a kind of magic when you think how much we talk. It's the community not the communication that matters, we need to feel connected, which is exactly the power of the internet.

So here is the project https://git.sr.ht/~ninjapanzer/krappy-dyndns

The assumption is that if you own a domain you likely also own some free hosting, really lame html hosting but a small piece of the internet that is yours as long as you pay for it. Kinda like a house and property taxes... but lets not go down that road. So your ISP gives you a ton of bandwidth so you can watch _Better Call Saul_ on Netflix but whats it doing when you aren't binging? Just idling like car insurance... but lets not go down that road either. Point is theres a lot of spare internet for the 50 people a month that are going to look at your website. That's pretty cool to be honest when you think about the number of people you might interact with on the average Friday at your local coffee shop. So here are the problems we need so solve:

- give your "special content" the impression its from a fixed location for the sake of discoverability
- find a normal way to allow a browser or application to call back from your internet house to your house house without being bungled by your ISP
- make it easy to maintain some services from your laptop or phone
- keep those things kinda working when those devices are offline

Yes, the idea is to host your site from your phone while its in your pocket. Crazy yes, possibly maybe, am I gunna try, yes.

So back to the point, you own some internet property and with the help of some krappy-dyndns we can publish a text file to the "free" hosting thats attached to your domain. This falls under the guise of what we call these days as .well-known. https://youraddress.com/.well-known/krappy-dyndns-8abe777a holding a binary stream of IP address histories and encoded with the name of a service. It's just an IP address and while its your IP address its also shared by others so its vaguely you. The daemon service runs on your target device and on an interval figures out what your IP is and then if it changes pushes it to that well-known file.

A user comes along and wants to leave a comment on your site. It makes a call to the comment service you run on your laptop and the client making the request knows what service it wants to interact with finding the correct .well-known and thus collecting an IP address. Next the tricky part, we have to trick your ISP to accept an incoming connection without an outbound call. Thats the whole NAT thing, probably utilizing something like https://en.wikipedia.org/wiki/Hole_punching_(networking). So your laptop will also host this service on your IP and allow for some underlying protocol like WebRTC to allow the initial transaction and boom the comment has been sent. Now, this is an internet that isn't trying to waste your time, so we take the comment and after its moderated we write it once back to our free hosting and if our laptop gets turned off for the night, who cares, people just cant leave a comment but the imporant stuff stays there. I mean they could always just send an email too.

Just one step in this crazy plan complete this week and another piece of the Krappy Internet is available.
