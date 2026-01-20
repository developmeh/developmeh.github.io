+++
title = "The AI Diaries"
template = "page.html"
weight = 0
draft = true
slug = "ai-diaries"
date = 2026-01-19
updated = 2026-01-19
[extra]
desc = ""
keywords = ""
enable_discussions = false
+++

First off this might sound like a shitpost, but anecdotally, I chuckle to myself about this all the time when I am vibe coding. Claude is something of a straw tiger here and the title is just for the lolz.

So the last couple of weeks I have been diving back into Claude-Code as my primary tool and away from Junie from Jetbrains. Under the hood I use Claude Sonnet for both and have been using various versions of gemmas and gippities since I stumbled across gpt4all like 3ish years ago.

I have a fun history with this technology, as a college student I naively tried to build this kind of knowledge based query interface in what I called the LDOCS, or Large Document Search, yep I couldn't figure out acronyms back then either. The idea of the project was to enter all the works from Mark Twain and then ask some questions about the TCU, Twain Creative Universe. It was wide eyed and it didn't work but was enough for my senior thesis. So point is I been thinking about this space and what my expectations of it for quite a while.

Now enters a real thing that isn't some idealistic trash I dreamed up and I get to use it every day. Its pretty sweet we all know it.

But is it really ready to work? Maybe, but lets run down a few experiences over the course of a year and my takeaway as a 20 year career veteran. I know this kinda breaks form with my trying not to talk about the AI side of LLMs but for me this is more about the time vs cost consideration of using such tools I plan on tailor the remaining to focus on effectiveness of the tooling and the problems I try to solve in building things gooder.

Of the projects I have built with AI recently the ones that were are overwhelming success were these in terms of time saved and value added. These were not entirely vibe coded but AI pair programmed:
Biggest win:

https://github.com/developmeh/passkey-origin-validator

So this was soup to nuts the best experience I had with the model aside from some tribulations I will explain. The TLDR; of this project is to provide some simulation and testing tools for assumptions around the Passkey/WebAuthN Related Origin Request spec. Which is a draft decision which provides something akin to CORS (Cross-Origin Resource Sharing) for a passkey. Since a Passkey is designed to affix to a single domain there are cases where your business is many domains. This provisionally allows passkeys to work off their origin on a predefined set of other origins. 

The problem is taking this from draft spec to tool is interesting as the functional implementation is handled by the browser and I was too dumb to understand it correctly from the draft spec alone.

Why this matters, well you have a huge enterprise rolling out a new security method and they have a lot of origins. Iteration on the global surface of a big company is hard to orchestrate. So just like writing end to end tests we need some harnesses to help us get everything right the first time.
Why Gooder

Well I started with some C code from the Chromium project which has a complete implementation of the draft feature and I am rust at C. So I fed a set of C files into the model along with my goal to produce a Go command line. It nailed the command line part.

    Models do best on things they have a lot of examples of -- Me

Go is marketed as very simple and not clever so command lines follow very stable libraries and rules. It smashed all the stuff I asked for an even some things I didn't know I wanted. Like flags for files vs URLs to test against. Made me look smart right here:

https://github.com/developmeh/passkey-origin-validator/blob/master/cmd/passkey-origin-validator/cmd/validate.go#L26-L27

    If no domain is provided, it uses the default domain (webauthn.io). If the --file flag is provided, it reads from the specified file instead.`, -- Claude

So now I can test against real vs proposed configurations... Nice.

It got chunky on the implementation though. Even with a provided concrete code resource it consistently miss-understood the procedure we were translating. Often confusing hard to research terms like eTLD (effective top level domains) and would fail to group subdomains and ccTLDs (country code tlds) correctly. Think about the rules for identifying the eTLD of developmeh.com and developmeh.co.jp we all know what that is but it had to understand and then make an algorithm for that.

I had to do all the work here. We "paired" I would interpret and guide and it would take shortcuts to the wrong answer.

This is also where I learned the biggest lesson about these models and the tools that orchestrate them.

    The Model doesn't think it predicts -- Paul

Even though I can explain the solution and the architecture it still plans for something it knows and not something it doesn't. So it follows the pattern and doesn't do well with the esoteric. Albeit what we were doing wasn't esoteric, to its knowledge it was very abstract.

That wasn't a failure though, I wrote the part it couldn't comprehend and it helped with tests and documentation a fair tradeoff as its penchant for verbosity suites just like how it extrapolated the extra command line flags I would clearly want.
Outcome

If was less skilled I might have not noticed it didn't really solve the problem and I would have created tech debt. I produced more than I would have on my own but the task took 20% longer than if I had just smashed it out myself. But if I step back to the full picture of a better tool with better docs I clearly saved at least that on volume. So its a tradeoff in the end like most things. Thankfully I could take over on the hard parts and redirect the model when it took an obvious to me shortcut to the solution. In the next example that didn't happen.
Biggest Loss:

https://github.com/developmeh/webrtc-poc

While this isn't exactly the body of the work, because I was never able to complete it, this is an artifact of a manner I have found successful in solving a problem and using AI.

The brief here is I wanna use the WebRTC (Real-Time Communication) standard to establish a tunnel between the internet and a CGNATed(Carrier Grade Network Address Translation) device. As you can imagine I wanna use this to provide a tunnel from anywhere to a server running on my phone. I wanna do this without a fixed STUN (Session Traversal Utilities for NAT) resource. Well that's not exactly true, I want something a kind to lazy dynamic DNS but I was able to build that. The architecture was such that a server could be ephemeral in transacting high latency operations for a website it serves by relying on a cheap cache for content and an events queue. If you wanna hear more about that check this out https://developmeh.com/projects/krappy-internet/ and start a discussion on github.

So the project I posted above for webrtc was an intermediate artifact I used to help the model figure some things out. I had the model help me build that working poc of the most basic webrtc integration. I had it provide a lot of context in markdown files and referenced this project over in my actual work. Here is the thing, I am trying to build something entirely new and somewhat off standard. Its viable but complicated and needs vetting. That said because of the more abstract nature of the work the model tends to fall over and couldn't figure out how to orchestrate the server starting in the right order to establish the same handshake it did on the PoC which is rather frustrating.
Outcome

To say the least I spend a lot of money on this with remarkably low success. From this point you might here me say:

    AI is great just don't ask it to do webrtc or anything with a handshake -- Me

But the reality is the theme of commonality of the task I was asking it to produce. That's where we should be trying to understand the models place in our work-streams.
The New Junior Dev

Yes and definitively, no. A more accurate representation is any dev the first time they work on a new project. I say this because I have seen seniors who are newly introduced to a new code-base make the same general mistakes the model does. I call them "shortcuts" because it appears they are skipping the good process racing towards the goal so they can go home, something like Mr Meeseeks(IYNYN).

I think this might be an argument to stop treating the entire code-base as a RAG (Retrieval Augmented Generation) source and instead consider model orchestration trained on it. But I am really not informed enough yet to make such a judgement call. On the shop floor shortcuts are the problem and I see these couple of common pitfalls that lead to them.

    Convoluted business logic (special cases)

    Unfocused context (not enough files in the RAG)

    Test confusion

In the second case when complex code is modified by the model it tends to focus its changes on a single file that appears to be akin to LoB(Locality of Behavior) but when there is too much abstraction in a project the model doesn't have a reasonably common pattern to predict against and cuts the corner by doing something easier, like changing contracts and moving things to a central location. What I love about this kind of solution is that's exactly what people tend to do who have a lower quality to completion drive. I internalize this is the same motivation hierarchy the model prefers.

The last one, test confusion is my favorite, where regardless of if it wrote the test or the test was provided the model will over enough time opt to change the implementation to meet the test regardless of if the test is right or wrong. When I say this its nothing so mundane as the value was supposed to be true so it changes the implementation to always return true. It will instead add a conditional check to the implementation to force that value only when the test suite is running. Here we have an example of something even the most junior novice would never do.

    The model is VERY HUMAN and ETHICALLY UNAFFECTED -- Me

This is a pretty big learning, and should define our trust of its outputs.

I was asked recently by someone I greatly respect:

    Paul, you have managed an engineering team before and you know we try to not micromanage people. Should we micromanage the AI though?

Yes, and that's generally the narrative about agents. They require a lot of "refinement" for anything complex. The folks over at METR have some general statistics around tasks less than 30 minutes the agents tend to complete successfully 80% of the time. This of course says nothing to the quality of the outcome but done is done for many simpler tasks.

Pair coding on the other-hand is not so easy METR also says that as the task approaches an hour the success falls to 50%, which mirrors my experience. That means effective usage is to keep tasks small. The success was "make a CLI" and I'll do the hard work. The latter of course was a broader scoped request and the moving parts were bigger and I understood the work less.

My expectation is that for bigger projects were you and the AI restart work after each pivot its harder to keep cognitive track of time spent unless you actually write it down. Which is completely fine if the work gets done eventually. Currently the models are all cheaper than humans for the same amount of time so the hope is it does a lot of bonus work similar to my big success project.
The New Determinism

So you probably noticed I didn't address "Convoluted business logic (special cases)" well because I wanted to come back to my thoughts on the ethics, lies, and prompt confusion. There is a lot you can learn about the attention mechanism in the AI's architecture and the hacks we have come up recently to address those like RE2(Re-Reading) and RLM(Recursive Language Model) which focus on how the prompt is manipulated to help avoid either attention loss on the token chain or improve on the limited context window respectively. So those are complete worth a read but at its core they shine a light on the fact that prompting is like programming in systems which is an orchestration of deterministic triggers.

When to send an email, is to how to reinforce attention and is a trigger. Since we are talking about generative code here, getting better outcomes is the management of model determinism. Its essentially an esoteric programming language, and just a couple of words or a convoluted logic statement during prompting can send the output into a tailspin. While I don't have anything but anecdotal experience to back this up. I think that's why tasks less than 30 minutes are more successful. Anything that's sufficiently complex enough is easy to confuse with illogical statements.

I can completely point to where I made a small logic mistake in my initial planning which effectively burns the 10 prior minutes of prompting. I left a bug somewhere at the top of the function and now I need to take some time to debug it. Except, debugging is kind of starting over fresh every time as I can't always correct the existing conversation chain. This is something I expect to become more prevalent and when it does, its just programming.

But, when I point out the model takes "shortcuts" those are likely just the impact of this new determinism, something of an unseen force. If you can prepare the models desk with all the sharpened pencils and neatly stacked paper it can draw you beautiful graphs. All the task broken down into unambiguous statements. There is something about software engineering that is desperate for Waterfall mythologies.
So we are back to Waterfall?

Nope not even close, try again. I hear all the time that effective LLM use for code gen is about planning everything. Its about small tasks, and tool construction. Better to see it like this:

    Whats best for the model is also whats best for you as a dev, you know the things that don't seem to save time -- Me

The future of the model's success in this field is DevEx, or maybe LLMEx. If we take a step up the abstraction tree away from libraries and into tools that are artifacts its kinda like playing a civilization sim, as the player we have to research, "hammer", so our little meeples can get hammers and that's part of a skill tree where we need hammers to make swords... OK this might go wrong. But seriously, generally asking the LLM to do the work is the wrong solution. It's kinda meh at it, but building tools that are small and composable so it can be the orchestration engine, now you might have something. Hell if the tool is small enough maybe it can even build it. How do you make a system that's easy to manage. SRP (Single Responsibility Principle), you build interfaces and contracts that are consistent, Contract First, and you focus on composition over inheritance. You keep the patterns simple and try to repeat yourself when possible, like poetry.


