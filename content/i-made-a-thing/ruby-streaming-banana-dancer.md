+++
title = "Ruby Dancing ANSI Banana for Curl"
template = "page.html"
weight = 0
updated = 2025-01-27
+++

**Hey there, fellow coder! Ever seen a parrot dance in your terminal?** 🦜💃

If you've taken a trip to `parrot.live`, you know exactly what I'm talking about. It's quirky, it's fun, and yes, a parrot dances right there in your terminal. But what if I told you there's another dancing star in town? And it's not a bird. Meet the *Ruby Streaming ANSI Banana*! 🍌

Yeah, I did say banana. 

So, here’s the scoop. I was chilling, thinking about how much I enjoyed that dancing parrot, and a thought popped up: "Could I do this with Ruby? And maybe... not a parrot?" Fast forward, and ta-da, a dancing banana was born. It clears your terminal screen with some nifty ANSI tricks and then gets its groove on. It's like the parrot, but you know, it’s a banana... and it's Ruby.

Now, I hear you: "But... why a banana?" The real question is, why not? Coding isn’t just about solving serious problems; it's also about having a bit of fun, letting your hair down, and, occasionally, making fruit dance in your terminal.

The best part? If you're team CURL, you're just one command away from some smooth banana moves. The banana doesn’t just dance—it does so smoothly, with some chunk-encoded charm ensuring that every move is in sync, right in your terminal.

So, curious? Want to dive into some fruity fun? Swing by [ruby_streaming_ansi_banana](https://github.com/developmeh/ruby_streaming_ansi_banana) and see it for yourself. And if you’re feeling extra creative, why not customize it? Maybe a shimmying strawberry or a waltzing watermelon?

Bottom line: In the world of code, there's always room for a dance, even if it's just a banana showing off its moves. So, let's not take ourselves too seriously and enjoy the rhythm, one ASCII character at a time!

---

Hope your keyboard’s ready for some dancing fun! 🍌🕺🎵

![dancing-banana](../dancing-banana.gif)

## DevLog

### 27 01 2025
#### Nix and planning for ruby streaming

I gotta admit I love nix a lot. Its the underdog to docker and even so I use it to create docker images. Although this is kind of a weird thing because when you consider nix you don't really need containers. Ultimately, nix was an alternative view of container runtimes. That said we have k8s and that is a container orchestration tool. That means I use nix to define consistent builds that produce docker images.

Lets walk back what using nix with something like ruby is like. I have previously done this with golang and that was a fun path https://git.sr.ht/~ninjapanzer/krappy_kafka/tree/main/item/flake.nix#L41-59

There we have a buildGoApplication extension to package a binary. That is rather easy because once done we execute that binary and orchestrate any file systems required.

In ruby we don't have a compile phase so we need to carry our bundled baggage. In this world we take the artifacts from bundler and describe them as nix store resources. Those are then copied to the container. In this world even the ruby version is part of the bundled nix context. Within the container we find these in the /nix/store path.

What I learned is that everything eventually is sourced from the nix store. Whats nice is we don't need bundler anymore since the path around the ruby runtime mixed with the context of our gems. While locally we might need to run **bundle exec ruby ...** now we use **"${gems.wrappedRuby}/bin/ruby** as our cmd.

Since this is my second time building a docker image for an arbitrary project, it feels less confusing. The big difference is where the build phase happens. In golang we build and then create an image. In languages like ruby we prepare our deps and then do build operations while copying files to root.

This really makes sense since we do the exact same thing outside nix. In go I would have a make operation that builds our binary and our dockerfile copies that file to the image.

With ruby we tend to bundle within the docker image creation. That is where this is magic, we don't do that anymore and as a result the docker image creation is super fast. Since we don't have to re-bundle on each build its easier to cache the nix env for our gems and the operation becomes one of copying instead of re-downloading and possibly compiling them. This eliminates the __compiling__ we are used to in the ruby space.

![xkcd compiling](../compiling.png)

Sorry guys, gotta keep working now
