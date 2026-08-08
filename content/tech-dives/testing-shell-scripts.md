+++
title = "Testing Shell Scripts: BATS, TAP, and Treating Bash as a Runtime"
template = "page.html"
weight = 0
draft = false
date = 2026-08-08
updated = 2026-08-08
slug = "testing-shell-scripts"

[taxonomies]
topics = ["Shell Testing", "Testing", "Developer Experience"]

[extra]
schema_type = "TechArticle"
desc = "A working guide to testing bash: what BATS is, why TAP is a reporting protocol rather than a test framework, and how shell function resolution order gives you mocking without a mocking library."
keywords = "bash testing, shell script testing, BATS, Bash Automated Testing System, TAP, Test Anything Protocol, mocking bash commands, stubbing sh, bats-assert, bats-mock, CLI integration testing, export -f, command builtin, shell spies, testing bash in CI"
sitemap_priority = "1.0"
sitemap_changefreq = "monthly"

[[extra.faq]]
q = "Can you write tests for bash scripts?"
a = "Yes. BATS, the Bash Automated Testing System, is a TAP-compliant test runner written in bash, with assertion and mocking helper libraries. The common claim that shell scripts cannot be tested is a cultural habit rather than a technical limitation."

[[extra.faq]]
q = "What is TAP, the Test Anything Protocol?"
a = "TAP is a reporting format and a way of consuming test results, not a testing framework. It describes how results are emitted so that any producer can be read by any consumer. BATS is TAP-compliant, which is why its output drops into CI tooling without adapters."

[[extra.faq]]
q = "How do you mock a command in a bash test?"
a = "Define a shell function with the same name as the command and export it with export -f. The shell resolves functions before it searches PATH, so every call site inside the script under test hits your function instead of the real binary. No mocking library is required, because the test framework is itself bash."

[[extra.faq]]
q = "How do you call the real command from inside a stub?"
a = "Use the command builtin. command helm3 \"$@\" bypasses function lookup and forces a PATH search, which turns a stub into a spy: you record the arguments, forward the call, and let the real side effect happen."

[[extra.faq]]
q = "How do you verify a step you cannot observe, like a temp file that gets deleted?"
a = "Stub the commands that hide it. Replacing mktemp lets you control and record every temp path the script creates, and replacing rm lets you copy each file aside before deletion. The intermediate state then becomes assertable without changing the script under test."

[[extra.faq]]
q = "Should you use a BATS mocking library or hand-rolled stubs?"
a = "Libraries such as bats-mock provide a DSL over the same underlying mechanism. For most cases a plain shell function plus export -f is enough and uses language you already know. Reach for the library when you want its richer expectation syntax."

[[extra.faq]]
q = "What should shell tests actually cover?"
a = "The boundary where the tool is used: argument handling and defaults, which external commands were invoked and with what arguments, files created or removed, exit codes, and behaviour under timeouts and concurrency. Unit tests in the implementation language cannot answer whether the thing works the way it is meant to be used."
+++

# Testing Shell Scripts

Bash has a reputation problem. It is the language people write when they cannot figure out how to do something in a "real" language, the duct tape holding CI together with `set -e` and crossed fingers. The received wisdom is that shell scripts do not get tested, because how would you even do that.

That is wrong, and it has been wrong for a long time. This page is the through-line across the shell and testing work on this site: [BATS: Testing Bash Like You Mean It](/tech-dives/bats-testing-bash-like-you-mean-it/), [TAPS: Not just a reporting protocol](/tech-dives/test-anything-means-testing-bash/), and [The Magic of Stubbing sh](/i-made-a-thing/the-magic-of-stubbing-sh/).

## Why does shell testing matter?

Bash is core to every Unix-like operating system. It is the glue between tools and the orchestration layer for a lot of distributed systems. If you are building CLI tools meant to be composed, piped, and chained together, bash is not a workaround. It is the runtime.

Which sets up the actual argument for testing it. I built a distributed job queue CLI whose components were solid Go with good unit tests. Those tests could not answer the question that mattered: does this thing work when you use it the way it is meant to be used? In bash, from the command line, with real files and processes and timing.

**Test at the boundary where the tool is actually used.** If that boundary is a shell, the integration tests belong in a shell too, rather than in Cucumber or a framework that spawns a browser.

There is a second reason, less about correctness and more about the conditions of the work. A lot of shell only runs in CI, on an OS that is not the one on your desk, using flags that differ between GNU and BSD. Verifying before you push is the difference between one iteration and eleven.

## What is BATS?

BATS, the Bash Automated Testing System, is a TAP-compliant test runner for shell. It runs tests, reports results, and has assertion helpers that are pleasant to use.

Three companion libraries do most of the ergonomic work:

- **bats-support**, required by the others
- **bats-assert**, deep assertion support such as `assert_line`, `assert_success`, `assert_output --regexp`
- **bats-mock**, a DSL for stubbing

The install advice that has served me best is to skip package managers, clone the repos into the project at a pinned depth, drop their `.git` directories, and commit the result. Everyone then runs the same version, and CI does not need network access to fetch a test framework.

A test looks like this:

```bash
@test 'when timeout is provided it will be set' {
  run sh ./helm.sh 18m

  assert_line --partial "--timeout 18m"
  assert_line "helm3 executed"
  assert_success
}
```

`run` executes the subject and captures output, status, and lines, so assertions are made against what the script actually emitted.

## What is TAP, and why is it not a test framework?

The Test Anything Protocol is worth understanding precisely, because the name misleads almost everyone who meets it.

TAP does not test anything. It is a **reporting format** and a manner of consuming test results. It specifies how results are emitted so that any producer can be read by any consumer. BATS being TAP-compliant is why its output drops into CI tooling without a bespoke adapter.

The name does suggest something better than what it is, and the aspiration is worth stating because it is a genuinely good idea. Consider what eBPF does for the kernel: attaching instrumentation to running privileged software without rebuilding it. **Test Anything** in that spirit would mean one interface for mocking and asserting against live running code, rather than a bespoke test framework and a pile of hard-to-read YAML per project. Symbols at runtime that can always assert against a running application. That is not what TAP is, but it is what the name should mean.

## How do you mock a command in bash?

This is the technique worth taking away from the whole page, and it is the reason testing shell is more pleasant than people expect.

**The shell resolves functions before it searches PATH.** So a function with the same name as a command shadows that command, for every call site, without touching the script under test:

```bash
function helm3() {
  echo "$@"            # capture the arguments
  echo "helm3 executed"
}

setup()    { export -f helm3; }
teardown() { unset -f helm3; }
```

`export -f` puts the function into the environment of child shells, which is what makes it visible to the script being run. `unset -f` in teardown matters, because a leaked stub in a reused shell produces a test that passes for the wrong reason.

**Because the test framework is itself bash, you do not need a mocking library.** There are good ones, and `bats-mock` is a fine DSL, but they are describing this same mechanism. For most cases a shell function is enough, and it uses language you already know rather than a second vocabulary.

### Turning a stub into a spy

When you want to observe a call without preventing it, `command` bypasses function lookup and forces a PATH search:

```bash
function helm3() {
  echo "$@"
  command helm3 "$@"   # the real binary still runs
  echo "helm3 executed"
}
```

Now the call is recorded and the real side effect still happens. The same builtin is what you use in teardown when your stub of `rm` would otherwise interfere with cleaning up.

## How do you verify steps you cannot see?

Some commands are hostile to observation. If a script creates a temp file and then deletes it, the interesting intermediate state is gone by the time the test looks. Writing traces to stderr proves only that the `echo` ran.

In a typical language you would reach for a mock or a spy that intercepts the call site through reflection. The shell equivalent is to stub the commands that hide the state, described at length in [The Magic of Stubbing sh](/i-made-a-thing/the-magic-of-stubbing-sh/).

**Stub `mktemp`** so the test chooses the paths, and record each one:

```bash
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

**Stub `rm`** so every file is copied aside before it disappears:

```bash
function rm() {
  for arg in "$@"; do
    if [[ "$arg" != -* ]]; then
      cp "$arg" "${TEST_DIRECTORY_RUNNING}/tmp/$(basename "$arg").captured" || return 0
    fi
  done
  command rm "$@"
}
```

The script under test is unchanged. The test now asserts on how many temp files were created, what each contained at the moment of deletion, and which files survived. That is the shell equivalent of a spy, built out of two functions and a counter file.

Anything bash can test, you can assert on: whether a file was created or updated, what a binary received, what a builtin returned.

## What else is worth testing?

Once the mechanism is available, the range is wide. Beyond arguments and defaults, [the BATS guide](/tech-dives/bats-testing-bash-like-you-mean-it/) works through shared setup in test helpers, asserting on JSON output, background processes, timeout behaviour, state machine transitions, time-dependent behaviour, and concurrent operations.

Those last few are where shell integration testing earns its place. A CLI that spawns a daemon, waits on it, times out, and cleans up is exactly the thing unit tests in the implementation language cannot reach.

## Where does this leave the argument?

"Shell scripts don't have tests" is a statement about habit, not capability. The tooling exists, it is TAP-compliant so it fits your CI, the mocking mechanism is the shell's own name resolution, and the tests read as the same language as the thing they test.

There is a side benefit worth naming. One way to learn an unfamiliar codebase is to read its tests, because tests use a more common vocabulary than implementation and tend to be more literate. Shell scripts with tests are shell scripts a newcomer can actually understand.

## Related reading

- [BATS: Testing Bash Like You Mean It](/tech-dives/bats-testing-bash-like-you-mean-it/), the full guide from installation to concurrency
- [TAPS: Not just a reporting protocol](/tech-dives/test-anything-means-testing-bash/), on TAP and a first BATS example
- [The Magic of Stubbing sh](/i-made-a-thing/the-magic-of-stubbing-sh/), hand-rolled spies for commands that hide their work
- [End User Languor Agreement](/terms-and-afflictions/eula/), on bespoke OS-specific bash and why CI-only scripts need verifying
- [CI Over CD](/devex/ci-cd/), on where testing sits in the delivery pipeline

Everything on this subject: [Shell Testing](/topics/shell-testing/), [Testing](/topics/testing/)
