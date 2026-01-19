+++
title = "The Perfect Development Environment"
template = "page.html"
date = 2025-01-28
updated = 2025-01-28
weight = 0
draft = false
discussion_number = 31
discussion_url = "https://github.com/orgs/developmeh/discussions/31"
+++

## The Perfect Development Environment

Let's be clear, this is all opinions and while this serves equally for those who focus on a single technology chain, it is optimized for those whom work on multiple projects with varied exacting dependencies and runtimes.

For example just in Ruby alone I may have a a legacy ruby 1.9 project on the same machine where I have multiple ruby 3.x projects. You might not see any conflict here and you would be right. Given each ruby project has a unique ruby version we don't really have any annoyances. But the moment I have 2 ruby 3.2 projects with various verisons of imagemagick I will find myself fighting. Of course this is related to a nuance of gems that bind to static libs and are somewhat opinionated about which exact version they need while that need is being provided by a system level package manager like Homebrew.

To be clear, I love homebrew, it made me who I am, but like JQuery its a product of an age that has passed.

What murders me is that Nix is 20 years old. I could have been using this the whole time, if it had any of today's features back then. But I am jumping the gun, lets continue with the targets of this project.

- Produce a template-able structure for any project
- Use open source tools that are well maintained
- Use patterns that make project adoption easier
- Management of dependencies must be project specific and avoid env collisions
- Leave artifacts behind that inform but don't require use

The guiding principle is _Leave artifacts behind that inform but don't require use_ we can't say this is done unless we can make this true. Everything before this makes its conditionality possible. If we do this work correctly we can allow the technological landscape to evolve and these techniques can be replaced with new superior solutions as they become vogue.

### Produce a template-able structure for any project

I have worked with a number of navel gazing developers who like to build walls around their languages and techniques marking them simultaneously superior and exclusive to their corners of the world. I would alike this to what has happened with protobuf and protoc in python.

#### A plea for protocol politeness

You can skip this section if you don't care about my personal experiences with protoc and python, while this is not a problem limited to protoc or python its a story of the smell produced by monoculture; something that no longer has a place in modern development.

In proper diatribe format our story is about the value of protocols and our common inability to avoid abstraction in the face of having to learn something new.

For those who have not used [Protobuf](https://protobuf.dev/) and its cli utility protoc (pronounced pro-toc), it has a rather simple protocol for adding extensions to its command line. Mind you python was not officially supported until sometime mid-2024 and all generators were community provided. Here is the catch like I attempted here [my grpc generator in rust](https://github.com/ninjapanzer/grpc_generator) required some funny incantations to get things working. At the time python developers wrapped protoc in a bespoke python library at the time those incantations would looks something like this `python -m grpc_tools.protoc -I. --protobuf-to-pydantic_out=. example.proto` and didn't publicly expose the actual plugin for protoc which expects something more like `protoc -I. --protobuf-to-pydantic_out=. example.proto`.

The protocol I speak of is the product of a clever CLI, __--protobuf-to-pydantic_out__ expects that in the current path is something executable (including a shell script) that goes by the name __protoc-gen-protobuf-to-pydantic__ so whatever is before __out__ must then be able to exist prefixed by __protoc-gen__. While somewhat poorly documented this protocol for extension makes it super easy to bolt on 1 or 10 plugins to build out a whole organizations worth of runtime specific artifacts.

Like I mentioned before '24 we had to do it the hard way and because the python communities view of DevEx and ergonomics is annoying binaries like protoc should be wrapped under the glaze of a python module.

So the lesson here is probably two fold. Firstly, I wasn't the only one who probably thought this whole python thing was dumb and by bringing python into the fold it has a spec plugin and now I don't have to worry about protocol violations to generate artifacts from protobuf IDL files. Secondly, its expected that when you produce abstractions for public consumption you are obligated to do so in observation of the authors protocol when wrapping their work.

Simplicity doesn't mean brevity, thus I am advocating for __Clarity__ over __Ease__. It should be easy to understand or do, so in terms of python authors overstepped here. We are going to try and do the same thing with our project layouts each piece will respect the patterns of its community regardless of expectations of the projects norms.

![xkcd standards](../standards.png)

Yep, I am thinking it too so just hang on.

### Back to the main event

What we want to inform on is how to consume a fresh project which usually has a few externals we should concern ourselves with. First of those are the runtime dependencies and here we have a ton of options. Just in my short life I have used all of the following:
- [rvm](https://rvm.io/)
- [rbenv](https://github.com/rbenv/rbenv)
- [nvm](https://github.com/nvm-sh/nvm)
- [maven wrapper](https://maven.apache.org/wrapper/)
- [gradle wrapper](https://docs.gradle.org/current/userguide/gradle_wrapper.html)
- [fnm](https://github.com/Schniz/fnm)
- [__sdkman__](https://sdkman.io/)
- [jenv](https://github.com/jenv/jenv)
- [__homebrew__](https://brew.sh/)
- [__apt/dpkg__](https://en.wikipedia.org/wiki/APT_(software))
- [__yum__](https://en.wikipedia.org/wiki/Yum_(software))
- [__pacman__](https://wiki.archlinux.org/title/Pacman)
- [__nix__](https://nix.dev/)
- [__asdf__](https://asdf-vm.com/)
- [__mise-en-place (mise) / rtx__](https://github.com/jdx/mise)
- [__ansible__](https://github.com/ansible/ansible)
- [pyenv](https://github.com/pyenv/pyenv)
- [virtual burrito](https://github.com/brainsik/virtualenv-burrito)
- [phpenv](https://github.com/phpenv/phpenv)
- [pvm](https://github.com/hjbdev/pvm)

While I am sure I forgot some you will notice two groups I have highlighted the ones that belong together. Whats different about these bold tools is they try to be the new standard of how to collect any runtime with some broad variances. I think this is a good place to start, but we should remember our goals and immediately eliminate those which don't project our project env from collisions with other projects. That means we say goodbye to all the package managers aside from __nix__ and __ansible__ albeit ansible has a special use case and is probably muddying the waters.

Of the remaining list we have __sdkman__, __asdf__, __mise__, and __nix__. Thats a pretty tight list lets go over how these work. The first three all do the same thing and will isolate each runtime in your home directory and shim you environment and each allows for a global system version and a config file driven variant per project folder albeit the format for sdkman is unique and asdf/mise are interchangeable in some cases. That leaves __nix__ which is our ugly duckling, as its syntax is rather obtuse so you need to have the right reason to use it for your project. That reason is probably more related to your build system then it is one of getting a runtime local to a project.

To be honest I don't generally advise using __nix__ to prepare your development environment runtime since it wants to own the whole environment using it to install say ruby means you also need to teach it how to install your gems. Which isn't horrible but might be overreaching, [an example with nix](https://github.com/developmeh/ruby_streaming_ansi_banana/blob/dba7eba58ccca137975a5a29ac720b6f5084cb32/flake.nix) while that also builds a docker image with the same context, that other reason you might wanna use __nix__ I mentioned, its pretty heavy compared to the competitors. Those primarily expose their configuration through something we expect a file that lists a runtime and a version. The tool then helps you install those versions on your computer and the configuration file acts as human readable documentation about the project in case your developer doesn't wanna use it.

#### Enough talk lets see something

Remember our opinion is that a git repo is just a folder and folders can live in folders, I don't wanna tell you to always put multiple projects in a repo or one project per repo so we only describe the project as a folder and where that folder lives is up to you.

```bash
/
├── .ci/
│   └── scripts/
├── .git/
├── .gitignore
├── .tool-versions
├── .deploy/
├── └── scripts/
├── .build/
├── └── scripts/
├── GETTING_STARTED.md
├── Makefile
├── README.md
├── src/
└── ...
```

I have seen variants of this generally where are scripts share one folder but generally I look at this from the approach of interfaces that are tool agnostic. That interface is exposed though make. Regardless of the actual build steps or build system, like bazel or nix. I should be able to say, __make build__ or __make deploy__ and I will get some feedback on how that is going.

The same is true with CI, which will probably be augmented by the dot file for your executor configuration be that CircleCI, Gitlab, Github, Sourcehut, or something else we will always need a place to hide some scripts and then bind them to make so our CI can also make the same calls that we might call like __make test__. The specific language for the cross project targets is outside the scope of this document but the three I stated should be a default with strong consideration for __make init__ or something to setup a first time run.

Thats not the only reason we want to put our scripts in their own project folders though. We want to be able to test them. I have become a huge fan of [BATS](https://github.com/bats-core/bats-core) as exampled [here](/tech-dives/test-anything-means-testing-bash/) each scripts folder can be extended with a tests folder for its given sub project like this without concern of polluting the scope of the actual codebase of the project.

```bash
/
├── .ci/
│   └── scripts/
│   └── tests/
```

__IF YOU EVEN ONCE SAY WE DON'T NEED TO TEST OUR BASH GET THE HELL OUT__ Test everything is TEST EVERYTHING!

So now if we have everything write we have a repo with a file to define its runtime dependencies like ruby or java that is explicit. If our project needs both, all the better.

Our makefile provides a common interface to declare activities and it mostly calls scripts in our various targets like build or deploy.

### Lets pick it apart

#### But I am using gradle
Sweet, gradle is cool and while you may consider that you can just run __gradle install__ instead of __make build__ we often have to bake extra commands and options to the actual build tool.

```make
.PHONY: build

build:
	@echo "Building with Gradle"
	@gradle install
```

Its not that hard and no one says you have to use it, but I bet after the 8th project you open that offers __make build__ and you don't even know if it is using bundler gradle or maven and you stop caring we have won.

#### But my golang project has a deploy module
Yea thats cool, thats why our project folders are all prefixed witha dot, just like git they are almost ephemeral if I deleted them all I wouldn't get a better project but the project would still be the project.

#### I don't need to deploy right now and my builds are uncomplicated
Great, the point is to define a template, if you don't need __.deploy__ don't use it. Same is true for __.build__ but when the decision comes, where should I put my scripts, this is the hint. If you need some other kind of special target for your project consider creating a special dot folder for it and give it some meaning while exposing it to make.

### Reasoning

At each level we are creating a little border around the tools and patterns we use, creating a project protocol if you will. So projects can be more interchangeable and better to keep them simple. We are intentionally saying "don't think about it just follow this pattern." While this might feel like overstepping into someone elses agency it should feel more like a relief because its a decision you don't have to make and ultimately you are not bound to. We should invite a repeatable protocol because being clever is like getting a puppy, its a lot of responsibility.

We started this discussion deep in the type of tooling but in reality this is about knowledge artifacts. We are trying to answer the following questions with our protocol:
- what version of x do I need to install
- how do I boot this up
- how do I deploy this
- how do I build this
- how do CI/Gitops/Automation happen

I have done all that without having to ensure someone already knows and better yet if they have seen a project they already know.

A template exists here for you consumption [Template](https://github.com/developmeh/the-perfect-project-template)

## That was the easy part

Now we need to address the reality of bigger projects, the stuff it needs from the OS to build complex projects.
(Coming soon)
