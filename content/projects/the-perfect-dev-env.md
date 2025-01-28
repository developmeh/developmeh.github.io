+++
title = "The Perfect Development Environment"
template = "page.html"
updated = 2025-01-28
draft = true
+++

## The Perfect Development Environment

Let's be clear, this is all opinions and while this serves equally for those who focus on a single technology chain, it is optimized for those whom work on multiple projects with varied exacting dependencies and runtimes.

For example just in Ruby alone I may have a a legacy ruby 1.9 project on the same machine where I have multiple ruby 3.x projects. You might not see any conflict here and you would be right. Given each ruby project has a unique ruby version we don't really have any annoyances. But the moment I have 2 ruby 3.2 projects with various verisons of imagemagick I will find myself fighting. Of course this is related to a nuance of gems that bind to static libs and are somewhat opinionated about which exact version they need while that need is being provided by a system level package manager like Homebrew.

To be clear, I love homebrew, it made me who I am, but like JQuery its a product of an age that has passed.

What murders me is that Nix is 20 years old. I could have been using this the whole time, if it had any of today's features back then. But I am jumping the gun, lets continue with the targets of this project.

- Produce a template-able structure for any project
- Use open source tools that are well maintained
- Use patterns that make project adoption easier
- Leave artifacts behind that inform but don't require use

The guiding principle is _Leave artifacts behind that inform but don't require use_ we can't say this is done unless we can make this true. Everything before this makes its conditionality possible. If we do this work correctly we can allow the technological landscape to evolve and these techniques can be replaced with new superior solutions as they become vogue.

### Produce a template-able structure for any project

I have worked with a number of navel gazing developers who like to build walls around their languages and techniques marking them simultaneously superior and exclusive to their corners of the world. I would alike this to what has happened with protobuf and protoc in python.

#### A plea for protocol politeness

You can skip this section if you don't care about my personal experiences with protoc and python, while this is not a problem limited to protoc or python its a story of the smell produced by monoculture; something that no longer has a place in modern development.

Today's diatribe is a story about the value of protocols and our common inability to avoid abstraction in the face of having to learn something.

For those who have not used Protobuf and its utility protoc, it has a rather simple protocol for extension. While even to this day I will get it wrong generally you can provide it a binary that accepts a set of arguments and it will fan out generation to each included plugin so
