+++
title = "The Krappy Internet"
template = "page.html"
date = 2025-01-29
updated = 2025-03-02
+++

## What if the internet stopped being shit and was instead Krappy?
The Krappy Internet is an attempt to re-envision how we trust data from the internet. This is barely even a hypothesis but in the pursuit of something closer to what the internet once was without bike shedding blockchains and onion routers I am building my own internet, just for me. Others can use it if it ever does anything.

### Components
Krappy Utils (In Progress) -> https://git.sr.ht/~ninjapanzer/krappy
Krappy Content Linker (In Progress) -> https://git.sr.ht/~ninjapanzer/krappy_internet
Krappy Navigator (Planned)

### Mircocosoms
In the beginning content lived on distinct domains which declared their purpose clearly in their domain name or the commonality of the content they maintained. Much like an address to a folder on a huge distributed computer hyperlinks created the connective tissue
between the content storage and meaningful reference. Even search engines only acted to provide a searchable inventory of those same links. In the early 2000s this modus changed in the drive to reduce barriers for users to publish their thoughts to the internet.
I don't know who to blame first but lets just say my earliest memory related to something like "Global Consciousness" a rather ugly site, appropriate for th time, where you could post a few worlds and it would show up for everyone. Kind of mind boggling the scale
of something like that back then. This wasn't the first though, as email was the first "social media" through usenets and before that bulleten board systems. The biggest difference between those early examples and what we have now is the nature of the silos created.
Content is restricted to a domain and distribution is controlled by the domain owners marketing budget at best, and at worst by the nefarious moderation of maddmen. The flow of information is best modulated at the consumer and not in the ivory towers of the board rooms.

When a content silo is generally healthy we will see an even discourse of thoughts and an opportunity to learn. The opposite is a self-reinforcing place where we can avoid the conflict of new ideas and further ambiguate reality. The need for critical thinking is personal
obligation of a democratic society.

### Nefarious Moderation

If I was to correlate discovery of knowledge with my youth some 30 years ago the challenge would be finding my way to he library and then finding the right book. The process very much aligns with the manner we extract data today but the moderation is opaque. I had
the choice of either using the "card catalog" or speaking to a "research librarian" to identify my resources. Both are somewhat expensive in terms of human expenditure but rely heavily curation and expertise. These two avenuse are aligned with search engines and wikipedia as
direct analogs a decade later. The value position of that system is a direct proportion to its speed and the agency of the curators to treat knowledge as uniform expression. This of course is the ideal and not all libraries were neutral, none could be free of inherant bias, and
thus are another form of imperfection. If we instead try to observe the form of the library and the librarian the intent is to act as a free store of knowledge, organized by consistent means and discoverable by the average human.

Moderation is at its core a kind of applied bias, one that slides towards societal norms. The locality of those norms is mediated by the range of human contact; in a town that was limited to hundreds and on the internet thats limited by language and discoverability.
Because a card in the catalog at the library has a fixed dimension there is also a limited topical granulatirty it can describe about an entry. Someone also has to use interpretation to categorize and prioritize those classifiers, another layer of invisible bias.
I want to believe that those involved take the role seriously but honestly, but I also know that this cannot be true, but I do believe that the default nature of people is to do good and those that do ill are a smaller portion of the whole. I expect that libraries
have been crowd sourcing classification for as long as they have existed. At some point the number of texts exceeds the capacity of the librarian to verify and we have to rely on publishers and other libraries to do the bulk work.

The same is true for content on the internet, but the value and classification has to benefit humans and not reinforce the dopamine factories. When we are rewarded for the sensational or rhetorical we assume a bias towards these topics and the value repeats instead
of grows. If we were to view "content" independent of "platform" and interacted with it as we would in a library, what would that card catalog look like? Who would fill out the cards? Who curates the summer reading list? The publisher or the librarian?

### Identity and Emergence

I adhere that you should put your name on things. I am American, and I with the mythology of figures like John Hancock, who's apocryphal heroism is laid out by signing his name large enough on the Decaration of Independence such that the landord could read it unassited.
Regardless of the veracity and accuracly of this take its influence what it means to "have a position" and "to express ones thoughts" where there is no place for anonymity. Its a bias allowed by my privilege, also I don't spend a lot of time in proximity to the emergence
of fact. So there is clearly a place for strong assumption of consistent identity and the emergence of information without a clear owner. The value is weighed by its validation, when giving credence to a statement it must have proof. Proof is well established through
consistency of action by a trusted identity, or by the expression of evidence. I wanna believe there is a place for investigative journalism's protected informants, for whistle-blowers, and for those fighting oppression to communicate. When the platforms are not aligned
with protecting the actors, which if you look at the long history of centralized platforms is under constant violation by state run organizations, hackers, and corporate greed I agrue no one is anonymous.

A person should be able to own whatever they publish, not by license but by attribution, you can prove you said it. You can also say it anonymously, since an identity is really analogous with trust an identity doesn't need to be a "person" but it should be "consistent".
Naturally, this means an identity can be an organization or a person, and content is aligned with that instead of their domain. Domains don't own identity they only hold content and act as addessable geographies. Many libraries carry the same books and in some cases
they trade those books with each other with decentralized ownership. But what can't change is the authors, the editors, and publishers, they are fixed and they act as the identities we assign or reject the proof of over time.

The value of identity is we can account for its duality, both the bad and the good are relatable and the only moderation will be self-moderation. Honestly, this is a really tricky subject, the lines were drawn long ago where accountability is a double edged sword. It
protects the mass from victimization and at the sametime subjects the part to possible ostracisation or harm. For now I like to think of identities as properties or assets. They are idempotent and addressable but not individual, an actor may have multiple identities. How
those identities assume trust and proof is based on the system that passively assigns it its trust.

While identities publish, it is the published material itself that is graded and the author doesn't receive immediate feedback about its reception. There are other networks and processes to be placed that help users collect and consume those publications wholly owned by
them.

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
