+++
title = "TAPS - Not just a reporting protocol"
template = "page.html"
weight = 0
draft = false
slug = "test-anything-means-testing-bash"
updated = 2025-01-27
+++

## Test Anything Protocol

So I rather love writing tests. Mostly because I don't understand my code and the code of the libraries I am implementing. But I sure as hell can understand the results. Maybe if there was a reason to write tests that would be it. I just kinda know I am dumb and its easy to write bugs so why not be a little sure. Recently, I was working in an unfamiliar codebase with a completely familiar command language ba_sh_. I wanted to be sure as I iterated thought a series of changes, ones that inevitably can't run on my machine and only in CI. When you take into consideration [DEVELOPER EULA](/terms-and-afflictions/eula) regarding bespoke OS specific bash commands this starts to make sense why you might want to just double check your code works.

Similarly in Ruby and other typeless languages the developer takes on the role of the compile time checker as well as feature writer. If that makes you wonder how they get anything done and write tests, the answer is as long as no one ever leaves the project things are going to be fine. So while I don't know why people still argue about if they should be writing tests and doing test driven development, all I can say is, lots of normal things are confusing. You know what I am talking about, climate change deniers, flat earthers, anti-vaxers, the over-woke(Sleepless in Seattle...).

Here is the point, when I got around to the part of the work where I was like, do I really wanna test this in production? Cowboy hat in hand, I thought, _Never drive black cattle in the dark_. So I took my good old time and asked the stars for guidance and what did I find? [BATS](https://github.com/bats-core/bats-core) which led me to a curious mistake. [TAP](https://testanything.org/), Test Anything Protocol, and I find out it doesn't test anything, in reality its a test reporting format and manner of consuming the results of tests, a protocol if you will. So that's all the history, but its what it inspired in me that brought me joy.

I don't know if you are familiar with [ePBF](https://ebpf.io/what-is-ebpf/) which is related to why [Crowdstrike broke the internet](https://en.wikipedia.org/wiki/2024_CrowdStrike_incident) that one day in '24. So here is what I wanted TAP to be, ePBF is a tech that lets you run ane extend software running with privilege, you know like kernel extensions that control your Windows System Security at the Airport. Oh yea they don have it because of some interesting non-competition reasons... (cough) Greed. Ok sorry, Test Anything, means to me we have a single interface and mechanism for mocking and asserting our running code. Imagine we don't have to have a bespoke test framework with gads of hard to understand YAML files in your go project. Instead we just have symbols at runtime that can always test a live running application. Its outlandish sure, but a guy can dream right. I mean its cool, so back to BATS which is pretty cool.

### Let's look at a quick example script

__helm.sh__

```bash
#!/usr/bin/env bash

DEFAULT_TIMEOUT=30m
TIMEOUT="${1:-$DEFAULT_TIMEOUT}"

helm3
--wait
--timeout ${TIMEOUT}
```

Pretty easy, we snag the first arg or provide the default. I have probably done this a dozen ways over the years but often skipped setting up any kind of testing. Really, this has just been good luck and the fact that these kinds of scripts are often small and rarely touched.

### Setup BATS

__install-bats.sh__
```bash
#!/bin/bash -e

if [ -d "./test/bats" ]; then
  echo "Deleting folder $FOLDER"
  rm -rf "./test/bats/"
  mkdir -p ./test/bats
else
  mkdir -p ./test/bats
fi

git clone --depth 1 https://github.com/bats-core/bats-core ./test/bats/bats
rm -rf ./test/bats/bats/.git
git clone --depth 1 https://github.com/ztombol/bats-support ./test/bats/bats-support
rm -rf ./test/bats/bats-support/.git
git clone --depth 1 https://github.com/ztombol/bats-assert ./test/bats/bats-assert
rm -rf ./test/bats/bats-assert/.git
git clone --depth 1 https://github.com/jasonkarns/bats-mock.git ./test/bats/bats-mock
rm -rf ./test/bats/bats-mock/.git
```

Here we dump the bats under a central _test_ directory and we include all the libs:
- [bats-support](https://github.com/ztombol/bats-support) - required for other libraries
- [bats-assert](https://github.com/ztombol/bats-assert) - adds deep support for asserts
- [bats-mock](https://github.com/jasonkarns/bats-mock) - allows for stubbing

### The Test

__helm.sh.bats__
```bash
#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

# Load Bats libraries
load ../test/bats/bats-support/load
load ../test/bats/bats-assert/load

function helm3() {
  # Captures and echos all the arguments each time helm3 is invoked
  echo "$@"
  echo "helm3 executed"
}

setup() {
  # export -f allows the function to be exported into the current shell env
  # What's cool about this is the shell looks for functions before commands
  # So if we have helm3 installed or not during the test this will be resolved first
  export -f helm3
}

teardown() {
  # unset is quite important if this shell is to be reused
  unset -f helm3
}

# Test cases
@test 'when timeout is provided it will be set' {
  # The first step is to run our script so bats can capture its output and setup the env for
  # our assertions
  run sh ./helm.sh 18m

  # allows us to assert a line and verify if any line in the output contains (--partial)
  # our expected string
  assert_line --partial "--timeout 18m"

  # a catchall to verify we called our stub and as we expect
  assert_line "helm3 executed"

  # asserts that the command exited with a 0 exit code
  assert_success
}

@test 'when timeout is not provided it will be the default' {
  run sh ./kube/install.sh

  assert_line --partial "--timeout 30m"

  assert_line "helm3 executed"
  assert_success
}
```

So thats it, you can test a bash script and mock the commands that we want to verify.

Of course we can also introduce a spy in the case we don't want to mock _helm3_

```bash
function helm3() {
  # Captures and echos all the arguments each time helm3 is invoked
  echo "$@"
  # Forces a PATH search and forwards arguments
  command helm3 $@
  echo "helm3 executed"
}

export -f helm3
```

Will allow the following execution:

`$ helm3 "HI"`
1. Will call the helm3 function
2. Echo the args
3. Call the helm3 command from the PATH
4. Echo our status message

In some cases you don't want your test to execute destructive operations but inspect its assumptions. Other times you need to know something happened but don't want to interfere with it. Because run captures all outputs we formulate our assertions around verifying those lines
produced in the output that are meaningful.

Here we have only explored interacting with arguments but its possible to assert anything that bash can test. If a file was updated, if a file was created, ultimately if a binary or built-in command holds our context for a valid assertion we can verify it.

Its not quite _Test Anything_ but its damn close.
