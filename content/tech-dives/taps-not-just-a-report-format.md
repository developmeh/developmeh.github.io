+++
title = "TAPS - Not just a reporting protocol"
template = "page.html"
weight = 0
draft = true
slug = "test-anything-means-testing-bash"
+++

## Test Anything Protocol

So I rather love writing tests. Mostly because I don't understand my code and the code of the libraries I am implementing. But I sure as hell can understand the results. Maybe if there was a reason to write tests that would be it. I just kinda know I am dumb and its easy to write bugs so why not be a little sure. Recently, I was working in an unfamiliar codebase with a completely familiar command language ba_sh_. I wanted to be sure as I iterated thought a series of changes, ones that inevitably can't run on my machine and only in CI. When you take into consideration [DEVELOPER EULA](@/terms-and-afflictions/eula) regarding bespoke OS specific bash commands this starts to make sense why you might want to just double check your code works.

Similarly in Ruby and other typeless languages the developer takes on the role of the compile time checker as well as feature writer. If that makes you wonder how they get anything done and write tests, the answer is as long as no one ever leaves the project things are going to be fine. So while I don't know why people still argue about if they should be writing tests and doing test driven development
