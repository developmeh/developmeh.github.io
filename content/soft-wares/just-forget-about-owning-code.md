+++
title = "Just Forget About Owning Code"
template = "page.html"
weight = 0
draft = false
date = 2026-02-03
[extra]
updated = 2026-02-03
desc = "LLMs and the death of bespoke code - why we should embrace composable, shared solutions instead of endlessly rebuilding the same things"
keywords = "LLMs, code ownership, open source, software architecture, FOSS, enshittification"
+++

<img src="/soft-wares/0204crcv.jpeg" alt="The future is FOSS" style="width: 100%; height: 600px; object-fit: cover; object-position: center calc(20% + 50px);">

## Just Forget About Owning Code
### Why keep making versions of the same thing?

So let's think about how LLMs are trained. I have, mostly because I have been reading [Build a Large Language Model (From Scratch)](https://sebastianraschka.com/llms-from-scratch/) and I was reminded of the nature of supervised / deep learning systems and their implication on how models are refined. Let's think about how LLMs got to this point, using this Wash Post article as an idea [Destroying and Scanning Books](https://www.washingtonpost.com/technology/2026/01/27/anthropic-ai-scan-destroy-books/), well it needs stuff to read and according to this article to get some of this volume it cut the bindings off books and scanned them. What the LLM produces is a highly advanced predictive generation of those sources. It's completely true that the model doesn't quite know the source of the information after training and because it's a sophisticated predictive engine it does better when creating something similar to what it trained on.

Ok so let's walk that back a little bit about code and that big engineering dream of generalizing solutions. To this point one thing LLMs do a great job of is creating CLI applications in GO, no surprise there are lots of examples of really good CLIs in GO. Of course some of this can be generalized to other languages and if I walk a few steps from here there is an argument to be made that designing literate CLI APIs is kinda solved. Sweet, as an engineer, I consider this a complete win as most of my work is purposely to offload knowing how to do things cause I have lots to do.

I can recall back in the days in the early web when pagination of post counts was a hard problem, now there is probably a go to framework for every language. Most of us don't really think much about pagination anymore, more we consider the kind of pagination we want and apply the solution.

But as things become more complicated the generalizations are too hard and have too many edge cases, solving this would take more time and money than even some community funded altruism would allow. Just consider Authentication, I have worked in a lot of places, and regardless of the agreed rule, "don't roll your own auth," sure as hell every one of these places has done just that. I can enumerate all the great FOSS auth platforms that could be used and extended, that aren't. Honestly, don't get me started on the nature of buy vs build vs vendor vs OSS, its the stupidest discussion you will ever hear. With LLMs it might even be dumber honestly, but this is the baseline for my argument.

Why isn't the advent of LLMs just the start of fluent FOSS solutions to all the things we repeatedly build, a reduction and concentration of quality. As we all spend money on LLMs reintroducing the wheel and building everything fast and naively we could be defining protocols and refining specs. Where is the moat (the thing that keeps someone else from running in and eating your lunch), well, there never was one, code is essentially valueless. The moat for a software business was the product and the money it costs to just build common implementations that send some data somewhere else. What keeps someone from competing with you is that building software is hosting systems and building software is expensive.

Why keep building bespoke versions of anything. Sure the code is cheap before LLMs cheap innovation comes from encapsulation, if you use the Linux ecosystem as an example. An environment where a majority of the interactions are using tools designed in the 80s.

> DOTADIW, or "Do One Thing And Do It Well." - Unix Philosophy

So we still have to build this stuff but that can be the work in the end. We organize the systems and we build the technologies to act as a host and we orchestrate and we compose whatever tools we need on demand.

Here is my dream, think a package manager like Nix but easier to write that describes some interaction and a general UI that is baseline whatever your operating system is for simplicity. Now consider you want a movie ticket, so you do something like this:

```ruby
## Iron Lung Ticket Buyer

search "theater inventory for postal code 1111" => data
search "Iron Lung" => data -> show_data
get "Paypal" => payment
get "calendar" => filter
get "seat chart" => picker

compose show_data -> filter("this evening") => filtered
compose filtered -> picker OR select(2)
resolve payment -> prompt => tickets
```

So I don't need AMC to produce a website but maybe they want to, I don't care. What I do want is to figure out what shows for "Iron Lung" are playing and get tickets this evening at my local AMC. I want to execute this structure on my local machine because its pretty simple. I am essentially composing some expert systems to do some things I want. Those systems are packaged and they might do some local LLM work or use NLP (Natural Language Processing) but the act is simple and the theater gets their money the way I wanna pay it. They don't have to build a paypal integration and I get some tickets. But there isn't really a reason this needs to be more complicated than this and I probably don't need a cloud provider to maintain this interaction.

### I lost you, but you want this

I know I lost you here because it looks like I have built a programming language and I kinda have but really the syntax doesn't matter so much its instructing the orchestration of a package manager, there is no compilation. Some simple model just walks through these steps and uses modules that provide the interactions you requested. The heavy lifting is all managed by the common interactions.

When I think of enshittification and owning the means of production the LLMs that generate code is a two edged sword, sure a company can produce a lot of features and compete but also a nobody can disrupt that and it gets to a point where you are going to spend all your time making your moat deeper and wider with more code but the number of people building bridges over the moat increases at a rate faster than you can defend.

In the case above Paypal is incentivized to create a module that allows their payment system to adapt to whatever the vendor supports. Either deeply integrated or using a one time credit card, it's now insanely easier for them to build that expert system and they have to compete with Stripe doing the same thing. The model shifts away from them locking in merchant rates but being the chosen consumer brand because they have the best tools or customer satisfaction.

The point is some business will not have a choice they don't have to build APIs anymore the web is the API and anyone can build code to extract that data.

### How does this not happen?

I am waiting for the time when the big AI companies start selling the ability to block certain types of code generation or they pass legislation that makes scraping a crime... think about it. We are nearing a case of mutually assured destruction or a human utopia.
