+++
title = "BATS - Testing Bash Like You Mean It"
template = "page.html"
weight = 0
draft = false
slug = "bats-testing-bash-like-you-mean-it"
date = 2026-02-08
updated = 2026-02-08
[extra]
desc = "Stop treating bash scripts like second-class citizens. BATS brings real testing rigor to CLI tools—from basic assertions to background processes and state machine validation."
keywords = "BATS, bash testing, CLI integration testing, shell script testing, automated testing, integration tests, bash automation, test helpers, mock commands, asynchronous testing"
discussion_number = 48
discussion_url = "https://github.com/orgs/developmeh/discussions/48"
+++

Bash has a reputation problem.

It's the language people write when they can't figure out how to do something in a "real" language. It's duct tape. It's the thing that holds your CI/CD together with `set -e` and crossed fingers. Nobody tests bash scripts because, well, how would you even do that?

This is bullshit.

Bash is core to every Unix-like operating system. It's the glue between tools. It's the orchestration layer for distributed systems. If you're building CLI tools meant to be composed, piped, and chained together—bash isn't a workaround, it's the runtime.

I built a distributed job queue CLI. The components were solid Go with good unit tests. But unit tests couldn't answer the real question: does this thing actually work when you're using it the way it's meant to be used? In bash. From the command line. With real files and processes and timing issues.

BATS—the Bash Automated Testing System—turned out to be the answer. Not Cucumber. Not end-to-end frameworks that spawn browsers. BATS. Because if your tool lives in bash, your integration tests should too.

Here's how to use it.

## What BATS Actually Is

BATS is a TAP-compliant testing framework for bash scripts. It runs tests, reports results, and provides assertion helpers that don't make you want to throw your keyboard.

### Installation

Skip the package managers. Clone the repos directly into your project so everyone gets the same version:

```bash
# install-bats-libs.sh
#!/bin/bash -e

if [ -d "./.test/bats" ]; then
  echo "Deleting folder $FOLDER"
  rm -rf "./.test/bats/"
  mkdir -p ./.test/bats
else
  mkdir -p ./.test/bats
fi

git clone --depth 1 https://github.com/bats-core/bats-core ./.test/bats/bats
rm -rf ./.test/bats/bats/.git

git clone --depth 1 https://github.com/ztombol/bats-support ./.test/bats/bats-support
rm -rf ./.test/bats/bats-support/.git

git clone --depth 1 https://github.com/ztombol/bats-assert ./.test/bats/bats-assert
rm -rf ./.test/bats/bats-assert/.git

git clone --depth 1 https://github.com/jasonkarns/bats-mock.git ./.test/bats/bats-mock
rm -rf ./.test/bats/bats-mock/.git
```

> **Bash Note:** `[ -d "./.test/bats" ]` uses the single-bracket test command ([`test`](https://linux.die.net/man/1/test)) to check if a directory exists. The `-d` flag returns true if the path exists and is a directory. Single brackets are POSIX-compliant and work in any shell. The spaces inside the brackets are required—`[-d ...]` won't work.

Run it once, commit the `.test/bats` directory. Now your tests work the same everywhere.

Need to start fresh? Here's the cleanup script:

```bash
# clean-bats.sh
#!/bin/bash -e

if [ -d "./.test/bats" ]; then
  echo "Deleting folder $FOLDER"
  rm -rf "./.test/bats/"
fi
```

This gives you:
- **bats-core** - The test runner itself
- **bats-support** - Required dependency for other helpers
- **bats-assert** - `assert_success`, `assert_output`, `assert_line`
- **bats-mock** - Stubbing external commands

Run tests with the local binary:

```bash
./.test/bats/bats/bin/bats .test/*.bats
```

Or add it to your PATH in your test helper (we'll get to that).

## Level 1: Basic Command Testing

Start simple. Can your CLI run without exploding?

```bash
# .test/basic.bats
#!/usr/bin/env bats

# Load helper libraries
load bats/bats-support/load
load bats/bats-assert/load

@test "command exists and shows help" {
    run mycli --help
    assert_success
    assert_output --partial "Usage:"
}

@test "version flag returns version" {
    run mycli --version
    assert_success
    assert_output --regexp '[0-9]+\.[0-9]+\.[0-9]+'
}

@test "invalid command shows error" {
    run mycli not-a-real-command
    assert_failure
    assert_output --partial "Unknown command"
}
```

Run it:

```bash
bats test/basic.bats
```

You get TAP output:

```
 ✓ command exists and shows help
 ✓ version flag returns version
 ✓ invalid command shows error

3 tests, 0 failures
```

### Setup and Teardown

Tests need clean state. Use `setup()` and `teardown()`:

```bash
# .test/workspace.bats
#!/usr/bin/env bats

load bats/bats-support/load
load bats/bats-assert/load

setup() {
    # Create temporary directory for this test
    TEST_TEMP=$(mktemp -d)
    cd "$TEST_TEMP"

    # Initialize your tool's workspace
    mkdir -p .myapp
}

teardown() {
    # Clean up after test
    cd /
    rm -rf "$TEST_TEMP"
}

@test "creates job file" {
    run mycli jobs create "Do the thing"
    assert_success

    # Verify file was created in workspace
    [ -f .myapp/queue.jsonl ]
}
```

> **Bash Note:** `[ -f .myapp/queue.jsonl ]` uses `-f` to test if a regular file exists ([`test`](https://linux.die.net/man/1/test)). In BATS, the test passes if the command returns exit code 0 (true). If the file doesn't exist, the test command returns 1 and BATS marks the test as failed.

Every test gets a fresh `$TEST_TEMP`. No pollution between tests. No "but it worked on my machine" because you forgot to clean up.

### Assertions That Actually Help

The basic assertions you'll use constantly:

```bash
# assertion-cheatsheet.bash (not a runnable file, just reference)
run some-command

# Exit code
assert_success          # Exit 0
assert_failure          # Exit non-zero
assert_equal $status 2  # Specific exit code

# Output
assert_output "exact match"
assert_output --partial "substring"
assert_output --regexp '^[0-9]+$'

# Line-specific (0-indexed)
assert_line --index 0 "First line"
assert_line --partial "appears somewhere"

# Negation
refute_output "should not appear"
refute_line --partial "nope"
```

This is already more rigorous than most bash scripts get. You're testing real behavior, not mocking function calls.

## Level 2: Test Helpers and Mocking

Real CLI tools interact with other tools. They read files. They parse JSON. They have dependencies.

You need test helpers.

### Shared Setup in a Test Helper

```bash
# .test/test_helper.bash

# Shared test workspace setup
export TEST_WORKSPACE="${BATS_TEST_TMPDIR}/workspace"
export MOCK_BD="${BATS_TEST_TMPDIR}/bin/bd"

setup_workspace() {
    rm -rf "${TEST_WORKSPACE}"
    mkdir -p "${TEST_WORKSPACE}/.myapp"
    mkdir -p "$(dirname "${MOCK_BD}")"
    cd "${TEST_WORKSPACE}"
}

teardown_workspace() {
    cd /
    rm -rf "${TEST_WORKSPACE}"
}
```

> **Bash Note:** `${VAR}` and `$(cmd)` look similar but do completely different things. `${BATS_TEST_TMPDIR}` is [parameter expansion](https://www.gnu.org/software/bash/manual/html_node/Shell-Parameter-Expansion.html)—it retrieves the value of the variable. `$(dirname "${MOCK_BD}")` is [command substitution](https://www.gnu.org/software/bash/manual/html_node/Command-Substitution.html)—it runs `dirname` and captures its output. The braces in `${VAR}` are optional for simple names (`$VAR` works too) but required when concatenating: `${VAR}_suffix` vs the broken `$VAR_suffix`.

```bash
# .test/test_helper.bash (continued)

# Mock the 'bd' command that your CLI depends on
# Uses heredoc (<<EOF) to write a multi-line script to a file
setup_mock_bd() {
    local issues_json="$1"

    cat > "${MOCK_BD}" <<EOF
#!/usr/bin/env bash
case "\$1" in
    list)
        cat <<'ISSUES'
${issues_json}
ISSUES
        ;;
    show)
        # Return single issue based on \$2 (issue ID)
        echo '{"id":"'"$2"'","title":"Mock issue","status":"open"}'
        ;;
    *)
        echo "Mock bd: Unknown command \$1" >&2
        exit 1
        ;;
esac
EOF

    chmod +x "${MOCK_BD}"
    export PATH="$(dirname "${MOCK_BD}"):${PATH}"
}

# JSON assertion helper
assert_json_field() {
    local json="$1"
    local field="$2"
    local expected="$3"

    local actual=$(echo "$json" | jq -r "$field")
    [[ "$actual" == "$expected" ]] || {
        echo "Expected ${field}='${expected}', got '${actual}'"
        return 1
    }
}

# File content helpers
assert_file_contains() {
    local file="$1"
    local expected="$2"

    grep -q "$expected" "$file" || {
        echo "File $file does not contain '$expected'"
        return 1
    }
}
```

> **Bash Note:** `<<EOF ... EOF` is a heredoc ([Here Documents](https://www.gnu.org/software/bash/manual/html_node/Redirections.html#Here-Documents))—a way to embed multi-line strings. Variables like `${issues_json}` are expanded inside. Use `<<'EOF'` (quoted delimiter) to prevent expansion when you want literal `$` characters in the output. The `cat > file <<EOF` pattern writes the heredoc content to a file.

> **Bash Note:** `local` declares a variable scoped to the current function ([`local`](https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html#index-local)). Without `local`, variables are global and leak into other functions—a common source of test pollution. Always use `local` for function parameters and temporary values.

> **Bash Note:** `[[ "$actual" == "$expected" ]]` uses double brackets, a bash-specific conditional ([`[[`](https://www.gnu.org/software/bash/manual/html_node/Conditional-Constructs.html#index-_005b_005b)). Unlike single brackets, double brackets don't require quoting variables to prevent word splitting, support pattern matching with `==`, and allow `&&`/`||` inside the expression. The `|| { ... }` pattern runs the block only if the test fails—a compact way to handle errors without `if/else`.

Now your tests can load this:

```bash
# .test/sync.bats
#!/usr/bin/env bats

load bats/bats-support/load
load bats/bats-assert/load
load bats/bats-file/load
load test_helper

setup() {
    setup_workspace
}

teardown() {
    teardown_workspace
}

@test "syncs with bd issues" {
    # Setup mock bd command to return fake issues
    local mock_issues='[
        {"id":"abc-123","title":"Fix the widget","status":"open"},
        {"id":"def-456","title":"Refactor gizmo","status":"done"}
    ]'

    setup_mock_bd "$mock_issues"

    # Run your CLI that calls 'bd list' internally
    run mycli sync

    assert_success
    assert_output --partial "Synced 2 issues"

    # Verify the sync created local files
    assert_file_exist "${TEST_WORKSPACE}/.myapp/issues.json"
}
```

### Mocking External Commands

Your CLI probably calls external tools. Git. curl. jq. Whatever.

Mock them:

```bash
# .test/test_helper.bash (add to existing file)
setup_mock_git() {
    cat > "${BATS_TEST_TMPDIR}/bin/git" <<'EOF'
#!/usr/bin/env bash
case "$1" in
    rev-parse)
        echo "abc123def456"  # Fake commit hash
        ;;
    status)
        echo "On branch main"
        echo "nothing to commit, working tree clean"
        ;;
    *)
        exit 1
        ;;
esac
EOF
    chmod +x "${BATS_TEST_TMPDIR}/bin/git"
    export PATH="${BATS_TEST_TMPDIR}/bin:${PATH}"
}
```

Then in your test:

```bash
# .test/deploy.bats (excerpt)
@test "records git commit in metadata" {
    setup_mock_git

    run mycli deploy

    assert_success

    # Verify it captured the fake commit hash
    local metadata=$(cat .myapp/last-deploy.json)
    assert_json_field "$metadata" ".commit" "abc123def456"
}
```

### Testing JSON Output

CLI tools love JSON. Test it properly:

```bash
# .test/json-output.bats (excerpt)
@test "job status returns valid JSON" {
    # Create a job first
    run mycli jobs create "Test job"
    assert_success

    local job_id=$(echo "$output" | jq -r '.job_id')

    # Query job status
    run mycli jobs show "$job_id"
    assert_success

    # Validate JSON structure
    echo "$output" | jq . > /dev/null || fail "Invalid JSON"

    # Check specific fields
    assert_json_field "$output" ".job_id" "$job_id"
    assert_json_field "$output" ".state" "pending"
    assert_json_field "$output" ".title" "Test job"
}
```

This is real integration testing. You're not stubbing out JSON parsing—you're testing the actual output your users will see.

## Level 3: Background Processes and State Machines

Here's where BATS gets interesting.

Real CLI tools do async things. They wait for conditions. They poll. They recover from failures. They manage state transitions.

### Testing Background Processes

Say your CLI has a `--wait` flag that blocks until a job completes. How do you test that?

```bash
# .test/async.bats
#!/usr/bin/env bats

load bats/bats-support/load
load bats/bats-assert/load
load bats/bats-file/load
load test_helper

setup() {
    setup_workspace
}

teardown() {
    teardown_workspace
}

@test "waits for job completion" {
    # Create a pending job directly in the file system
    local job_id="job-$(date +%s)"
    local pending_job='{
        "job_id":"'${job_id}'",
        "title":"Background test job",
        "state":"pending",
        "created_at":"'$(date -Iseconds)'"
    }'

    echo "$pending_job" > "${TEST_WORKSPACE}/.myapp/queue.jsonl"

    # Start the wait command in the background
    mycli jobs show "$job_id" --wait --timeout=10s \
        > "${TEST_WORKSPACE}/output.txt" 2>&1 &
    local wait_pid=$!

    # Give it a moment to start
    sleep 1

    # Simulate job completion by moving it to done state
    local completed_job='{
        "job_id":"'${job_id}'",
        "title":"Background test job",
        "state":"completed",
        "created_at":"'$(date -Iseconds)'",
        "completed_at":"'$(date -Iseconds)'"
    }'

    rm -f "${TEST_WORKSPACE}/.myapp/queue.jsonl"
    echo "$completed_job" > "${TEST_WORKSPACE}/.myapp/done.jsonl"

    # Wait for the background process to finish
    wait $wait_pid
    local exit_code=$?

    # Verify it exited successfully
    assert_equal $exit_code 0

    # Check the output
    run cat "${TEST_WORKSPACE}/output.txt"
    assert_output --partial "completed successfully"
}
```

> **Bash Note:** The `&` at the end of a command runs it in the background ([Job Control](https://www.gnu.org/software/bash/manual/html_node/Job-Control-Basics.html)). `$!` is a special variable containing the PID of the last background process ([Special Parameters](https://www.gnu.org/software/bash/manual/html_node/Special-Parameters.html)). The [`wait`](https://linux.die.net/man/1/bash) builtin blocks until the specified PID exits and sets `$?` to its exit code. This pattern—background a process, do something, then wait for it—is essential for testing async CLI behavior.

> **Bash Note:** `$(date +%s)` uses command substitution ([Command Substitution](https://www.gnu.org/software/bash/manual/html_node/Command-Substitution.html)) to capture a command's stdout as a string. The `$()` syntax is preferred over backticks because it nests cleanly and is easier to read.

You're testing the actual polling logic, the actual file watching, the actual timeout behavior. Not a mock. Not a stub. The real thing.

### Testing Timeout Behavior

What happens when things don't complete?

```bash
# .test/async.bats (continued)
@test "wait times out if job never completes" {
    local job_id="job-timeout-test"
    local pending_job='{"job_id":"'${job_id}'","state":"pending"}'

    echo "$pending_job" > "${TEST_WORKSPACE}/.myapp/queue.jsonl"

    # Start wait with short timeout
    mycli jobs show "$job_id" --wait --timeout=2s \
        > "${TEST_WORKSPACE}/output.txt" 2>&1 &
    local wait_pid=$!

    # Don't complete the job - let it timeout

    wait $wait_pid
    local exit_code=$?

    # Should exit with error
    assert_equal $exit_code 1

    run cat "${TEST_WORKSPACE}/output.txt"
    assert_output --partial "timeout"
}
```

### Testing State Machine Transitions

Job queues are state machines. Jobs move between states. Some transitions are valid. Some aren't.

```bash
# .test/state-machine.bats (excerpt)
@test "prevents invalid state transitions" {
    local job_id="state-test-job"

    # Create completed job
    local completed_job='{"job_id":"'${job_id}'","state":"completed"}'
    echo "$completed_job" > "${TEST_WORKSPACE}/.myapp/done.jsonl"

    # Try to start a completed job (invalid transition)
    run mycli jobs start "$job_id"

    assert_failure
    assert_output --partial "Cannot start job in completed state"

    # Verify job state didn't change
    run mycli jobs show "$job_id"
    assert_json_field "$output" ".state" "completed"
}
```

### Testing Time-Dependent Behavior

The hard part. Jobs with heartbeats. Stale locks. Orphan recovery.

```bash
# .test/recovery.bats (excerpt)
@test "recovers orphaned jobs with stale heartbeats" {
    # Create job with old heartbeat (2 minutes ago)
    local stale_time=$(date -Iseconds -d '2 minutes ago')
    local job_id="orphan-job"

    local orphan_job='{
        "job_id":"'${job_id}'",
        "state":"running",
        "started_at":"'${stale_time}'",
        "heartbeat_at":"'${stale_time}'"
    }'

    echo "$orphan_job" > "${TEST_WORKSPACE}/.myapp/active.jsonl"

    # Run recovery command
    run mycli jobs recover

    assert_success
    assert_output --partial "Recovered 1 orphaned job"

    # Verify job moved back to queue
    assert_file_exist "${TEST_WORKSPACE}/.myapp/queue.jsonl"
    refute_file_exist "${TEST_WORKSPACE}/.myapp/active.jsonl"

    # Verify job state reset
    run mycli jobs show "$job_id"
    assert_json_field "$output" ".state" "pending"
}
```

This test manipulates time by creating timestamps in the past. It then verifies that your recovery logic correctly identifies stale jobs and transitions them.

### Testing Concurrent Operations

Multiple processes writing to the same files. The nightmare scenario.

```bash
# .test/concurrency.bats (excerpt)
@test "handles concurrent job creation" {
    # Start 5 job creations in parallel
    for i in {1..5}; do
        mycli jobs create "Concurrent job $i" &
    done

    # Wait for all background processes
    wait

    # Verify all 5 jobs were created
    run mycli jobs list
    assert_success

    local job_count=$(echo "$output" | jq '. | length')
    assert_equal "$job_count" "5"

    # Verify no duplicate job IDs
    local unique_ids=$(echo "$output" | jq -r '.[].job_id' | sort | uniq | wc -l)
    assert_equal "$unique_ids" "5"
}
```

If your CLI uses file locking or atomic writes, this test will catch races.

## Running Your Test Suite

```bash
# run-tests.sh
#!/usr/bin/env bash
set -euo pipefail

# Run all BATS tests using the local install
./.test/bats/bats/bin/bats .test/*.bats

# Or for more verbose output
# ./.test/bats/bats/bin/bats --tap .test/*.bats

# Or with timing
# ./.test/bats/bats/bin/bats --formatter tap --timing .test/*.bats
```

Make it executable:

```bash
chmod +x run-tests.sh
```

### CI Integration

If you committed the `.test/bats/` directory (recommended), CI is trivial:

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  bats:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run BATS tests
        run: ./.test/bats/bats/bin/bats .test/*.bats
```

No installation step needed. The test framework is already in your repo.

If you prefer not to commit the bats libraries, run the install script first:

```yaml
# .github/workflows/test.yml (alternative)
name: Tests

on: [push, pull_request]

jobs:
  bats:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install BATS
        run: ./install-bats-libs.sh

      - name: Run BATS tests
        run: ./.test/bats/bats/bin/bats .test/*.bats
```

Now every push runs your full integration test suite.

## Tips for Keeping Tests Fast

BATS tests are real integration tests. They're slower than unit tests. That's fine. But you don't want them to be slow.

**Don't repeat expensive setup:**

```bash
# .test/expensive-setup.bats (example pattern)

# SLOW - creates workspace every test
setup() {
    setup_workspace
    mycli init  # Expensive operation
}

# FAST - use setup_file for one-time setup
setup_file() {
    export SHARED_WORKSPACE=$(mktemp -d)
    cd "$SHARED_WORKSPACE"
    mycli init
}

teardown_file() {
    rm -rf "$SHARED_WORKSPACE"
}
```

**Use `--filter` during development:**

```bash
# Only run tests matching pattern
bats --filter "concurrent" test/*.bats
```

**Parallelize with `--jobs`:**

```bash
# Run tests in parallel (requires bats-core >= 1.5.0)
bats --jobs 4 test/*.bats
```

## When Not to Use BATS

BATS is for testing bash scripts and CLI tools. It's not for:

- Testing web UIs (use Playwright, Cypress, etc.)
- Unit testing Go/Rust/Python code (use your language's test framework)
- Load testing (use k6, Locust, etc.)

But if you're testing the actual user experience of a CLI tool—the thing someone runs from their terminal—BATS is perfect.

## The Point

Bash isn't a toy language. It's not "just scripts." It's the orchestration layer for most of the software infrastructure on the planet.

If you're building CLI tools meant to be composed and chained together, your integration tests should reflect that reality. Test them in the environment they'll actually run: bash, with real files, real processes, real timing.

BATS gives you the structure to do that without losing your mind. Setup and teardown that works. Assertions that read like English. Helpers that let you mock external dependencies without rewriting your entire tool.

Your bash scripts deserve tests. BATS makes it possible.

---

**Further Reading:**
- [BATS Core Documentation](https://bats-core.readthedocs.io/)
- [bats-assert helpers](https://github.com/bats-core/bats-assert)
- [bats-file helpers](https://github.com/bats-core/bats-file)
- [Test Anything Protocol (TAP)](https://testanything.org/)
