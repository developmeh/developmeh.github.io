+++
title = "The Magic of Stubbing sh"
template = "page.html"
weight = 1
draft = false
date = 2025-10-09
updated = 2025-10-09
[extra]
desc = "Testing sh is as easy as anything else, if testing is about side-effects its easy"
keywords = "BATS, TAPS, sh, bash, testing, mocking, stubbing, shell script, Bash Automated Testing System"
+++

## The Magic of Stubbing sh

I really love sh and bash but I often feel alone and I get some regular negativity when I solve a problem with it. I know why too, shell scripts can have a broad level of complexity that has other languages embedded into it. But its not as esoteric as you might think, more another domain we should be comfortable with. One of the ways I learned to deal with unknown domains was to read the tests. Because tests tend to use some common language they are often more literate. Here's the thing, I keep getting people tell me that shell scripts don't have tests, and they are wrong. See I have this trick, its called BATS and I talked about it over here [Test Anything Protocol](/tech-dives/test-anything-means-testing-bash) where I showed an example of stubbing `helm` but that example was not the whole story. Since the BATS framework is itself bash we have all those nasty tools at our disposal to manipulate our subject under test.

## Subject Under Test

Boring as it may be the purpose here is to observe and verify the output and side-effects of commands run by the shell. We need to respect this boundary between our scripts and the tests for those scripts. One of the challenges to this is how commands avoid observation like `rm` `mktemp`, if my script creates a tempfile and then removes it it’s hard to verify if that step occurred without modifying the subject. Of course we can write traces to `&>2` using echo but that proves nothing more than the presence of the echo statement. I need to verify the validity of these intermediate steps. In traditional programming languages we have mocks and spies which capture the fundamental flow of the code by interfering with the call sites and through reflection. We can do something similar.

## Mocking or Stubbing... Whatever
Now there are BATS mocking libraries and they are a wondrous cornucopia of features but in my experience they don't expose much more than a new way of describing, a DSL, how to intercept and modify interactions. So go learn and use those, but for many normal use cases I wanna show you how to do this by hand and use the existing shell language you already know. In the following example we are going to observe tempfiles so we can keep track of an intermediate state, while exposing debugging information when doing TDD, more on that down the line though.


### Example

__temp.sh__ Subject Under Test
```bash
#!/bin/bash -e

local workspace=$(mktemp -d)

touch "$workspace/not_temp.sh"

local first=$(mktemp)
local second=$(mktemp)

echo "WOW" > $second

rm $first
rm $second
```

__temp.sh.bats__
```bash
#!/usr/bin/env bats

set +x

bats_require_minimum_version 1.5.0

# Load Bats libraries
load ../../.test/bats/bats-support/load
load ../../.test/bats/bats-assert/load

# Stub rm to capture files deleted
function rm() {
  for arg in "$@"; do
    if [[ "$arg" != -* ]]; then
      cp "$arg" "${TEST_DIRECTORY_RUNNING}/tmp/$(basename "$arg").captured" || return 0
    fi
  done
  command rm "$@"
}

# Stub mktemp to track temp files for cleanup
function mktemp() {
  local tmp
  if [[ "$1" == "-d" ]]; then
    tmp="${TEST_DIRECTORY_RUNNING}"
  else
    read -r counter < $TEMPS_COUNTER
    ((counter++))
    echo $((counter)) > $TEMPS_COUNTER
    tmp="${TEST_DIRECTORY_RUNNING}/tmp/bats.${counter}"
    echo "$tmp" >> $TEMPS
  fi
  echo "$tmp"
}

setup() {
  export TEST_DIRECTORY="./.tests/res"
  export TEST_DIRECTORY_RUNNING="./.tests/res_tmp"
  export TEMPS_COUNTER=${TEST_DIRECTORY_RUNNING}/tmp/.counter
  export TEMPS=${TEST_DIRECTORY_RUNNING}/tmp/.temps
  cp -r "${TEST_DIRECTORY}/." "${TEST_DIRECTORY_RUNNING}/"
  mkdir -p "${TEST_DIRECTORY_RUNNING}/tmp"
  export -f mktemp
  export -f rm

  touch $TEMPS_COUNTER
  touch $TEMPS
  echo 0 > $TEMPS_COUNTER
}

teardown() {
  for tmp in "${temps[@]}"; do
    command rm -f "$tmp"
  done

  unset -f mktemp
  unset -f rm

  command rm -f "$TEMPS_COUNTER"
  command rm -f "$TEMPS"

  unset TEST_DIRECTORY
  unset TEST_DIRECTORY_RUNNING
  unset TEMPS_COUNTER
  unset TEMPS
}

@test 'test intermediate files' {
	local second_tempfile_expected="WOW"
  run bash ./.tests/temp.sh

  # note the captured
  local second_tempfile_actual="$(cat ${TEST_DIRECTORY_RUNNING}/tmp/bats.2.captured)"
  assert_success

  assert_equal $(cat "$TEMPS_COUNTER") 2
  assert_equal "$([ -f $TEST_DIRECTORY_RUNNING/not_temp.sh ] && echo 0 || echo 1)" 0
  assert_equal $second_tempfile_actual #second_tempfile_expected
  assert_output --regexp 'Done'

  # _Note_ The use of `command` which bypasses our function export of `rm` introduced by `export -f rm` this makes sure we use the original command and not our mock.
	command rm -rf "${TEST_DIRECTORY_RUNNING}"
}
```

Lets explore the mocking... ignoring the directory paths we intercept calls to mktemp and if the commands first argument is `-d` for directory we inject a static location we control. Otherwise we create a unique file in that directory. When we do this we capture the temp file and the number created so far so we can verify the interfaction later. Both these files can be observed during execution.

```bash
# Stub mktemp to track temp files for cleanup
function mktemp() {
  local tmp
  if [[ "$1" == "-d" ]]; then
    tmp="${TEST_DIRECTORY_RUNNING}"
  else
    read -r counter < $TEMPS_COUNTER
    ((counter++))
    echo $((counter)) > $TEMPS_COUNTER
    tmp="${TEST_DIRECTORY_RUNNING}/tmp/bats.${counter}"
    echo "$tmp" >> $TEMPS
  fi
  echo "$tmp"
}
```

When we write clean scripts we also clean up after ourselves this good behavior provides a challenge to checking the contents of these intermediate files. Because shell scripts are file system based the most common way for data to make its way between processes is to write and read from the filesystem. But if we are tracing a bug in our code we have to regularly interfere with out subject under test to observe its intermediate steps. But if we capture the `rm` command we can conditionally retain some of the progress. In this example we capture all the args and if one includes a path we extract the filename, append `.captured` and copy it to our running directory. Ultimately, even if we don't stub mktemp we can still capture deleted tempfiles this way.


_Note_ The use of `command` which bypasses our function export of `rm` introduced by `export -f rm` makes sure we use the original command and not our mock.
```bash
# Stub rm to capture files deleted
function rm() {
  for arg in "$@"; do
    if [[ "$arg" != -* ]]; then
      cp "$arg" "${TEST_DIRECTORY_RUNNING}/tmp/$(basename "$arg").captured" || return 0
    fi
  done
  command rm "$@"
}
```

Now lets review the test, first we can do traditional expectation with the assert module following the standard, Given, When, Then structure we love. Let's look at how the When is structured too, because this is bash whichever assertion fails the program will exit there. So note the last line where we clean up the temp directory for the test. By leaving this as the last statement we keep the test artifacts if the test fails. Which enables better TDD, where we write a test that fails and continue to iterate until that test passes, meanwhile the test is also producing trace and debugging information about our work. We can do this with any command though, say we call `git diff` and we want to verify what we produced. We can intercept any command and have it write a file to our test workspace. Importantly, while not changing the subject under test.
```bash
@test 'test intermediate files' {
	# Given
	local second_tempfile_expected="WOW"

  # When
  run bash ./.tests/temp.sh

  # Then
  local second_tempfile_actual="$(cat ${TEST_DIRECTORY_RUNNING}/tmp/bats.2.captured)"
  assert_success

  assert_equal $(cat "$TEMPS_COUNTER") 2
  assert_equal "$([ -f $TEST_DIRECTORY_RUNNING/not_temp.sh ] && echo 0 || echo 1)" 0
  assert_equal $second_tempfile_actual #second_tempfile_expected
  assert_output --regexp 'Done'

  # _Note_ The use of `command` which bypasses our function export of `rm` introduced by `export -f rm` this makes sure we use the original command and not our mock.
	command rm -rf "${TEST_DIRECTORY_RUNNING}"
}
```

## Just Test Things and Be Happy
This is just one dumb example of how to think about your testing and how to build up useful tooling that caters to your work. Now go write some bash and make sure you test it, trust me orchestrating a call to `git` is 10 times easier than screwing around with some git integration for your language of choice. These tools were meant to work together in the shell and you will be happier just getting things done. Double happy when you can prove it works with a test.

## Errata
### sh is not bash and vice versa
While not functionally errors, the title of this work should be focused on bash. Since a lot of the sample code are bash-isms especially _exported functions_.

### the sh alias and CI
> run sh ./.tests/temp.sh

`sh` is often an alias on modern systems and this can have a huge impact when you scripts run in CI or more namely a non-interactive or non-login session. Where you CI might offer an Ubuntu or Alpine Linux image that provides `bash` as an alias for `sh` it may use a lighter weight implementation like `dash` when running your tests. Because we are using features that are explicitly bash we should have our test suite `run bash ./.tests/temp.sh` as such I have altered the above example accordingly.
