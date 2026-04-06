// CLI Tools Section Content
(function() {
    const cliContent = `
        <div class="cli-overview">
            <p class="section-intro">
                Kwike provides a comprehensive CLI toolkit organized around the event pipeline lifecycle.
                Each command follows Unix philosophy: do one thing well, compose via standard protocols.
            </p>
        </div>

        <div class="cli-category">
            <h3>Core Pipeline Commands</h3>
            <p class="category-intro">Build event-driven workflows from production to consumption.</p>

            <div class="command-card">
                <h4><code>kwike daemon</code></h4>
                <p class="command-desc">Event store server and query interface. Owns the events.jsonl append-only log.</p>
                <div class="code-block">
<pre><code># Unix socket (default)
kwike daemon

# Custom socket path
kwike daemon --socket /path/to/daemon.sock

# HTTP server for remote access
kwike daemon --http :8080</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>--socket &lt;path&gt;</code> - Unix socket path (default: .kwike/daemon.sock)</li>
                    <li><code>--http &lt;addr&gt;</code> - HTTP server address (e.g., :8080)</li>
                </ul>
                <div class="command-note">
                    <strong>Mesh Routing:</strong> Configure namespaces and upstreams in kwike.yaml to route events across daemon boundaries.
                </div>
            </div>

            <div class="command-card">
                <h4><code>kwike dispatch</code></h4>
                <p class="command-desc">Emit JSON events from stdin to the daemon. Single events or JSON-lines streams.</p>
                <div class="code-block">
<pre><code># Single event
echo '{"id": "abc", "title": "Fix bug"}' | kwike dispatch --type beads.task

# Multiple events (JSON-lines)
bd list --json | jq -c '.[]' | kwike dispatch --type beads.task

# Dry run (preview without sending)
echo '{"test": true}' | kwike dispatch --type test.event --dry-run</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>--type &lt;event-type&gt;</code> - Event type (required)</li>
                    <li><code>--daemon &lt;url&gt;</code> - Daemon URL (default: local socket)</li>
                    <li><code>--dry-run</code> - Preview without emitting</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike watch</code></h4>
                <p class="command-desc">Poll a command periodically and dispatch its output as events.</p>
                <div class="code-block">
<pre><code># Poll every 30 seconds
kwike watch "bd list --json" --type beads.snapshot --interval 30s

# Watch with custom daemon
kwike watch "git status --json" --type git.status --interval 1m --daemon http://remote:8080</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>&lt;command&gt;</code> - Command to poll (quoted string)</li>
                    <li><code>--type &lt;event-type&gt;</code> - Event type (required)</li>
                    <li><code>--interval &lt;duration&gt;</code> - Poll interval (e.g., 30s, 1m)</li>
                    <li><code>--daemon &lt;url&gt;</code> - Target daemon</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike consume</code></h4>
                <p class="command-desc">Poll events, render uniform templates, supervise LLM subprocess, validate and ACK results.</p>
                <div class="code-block">
<pre><code># Normal operation (long-running)
kwike consume --config implementer.yaml

# One-shot (process one event and exit)
kwike consume --config implementer.yaml --once

# Dry run (render template without spawning LLM)
kwike consume --config implementer.yaml --once --dry-run</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>--config &lt;file&gt;</code> - Consumer config (required)</li>
                    <li><code>--once</code> - Process single event and exit</li>
                    <li><code>--dry-run</code> - Render template only (no LLM)</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike collect</code></h4>
                <p class="command-desc">Track completion events for fan-out/fan-in patterns. Reads events from stdin, persists state to disk, emits a collected event when all workers report.</p>
                <div class="code-block">
<pre><code># Initialize collection from a dispatched event
echo '{"type":"job.test.dispatched","thread_id":"T1","id":"evt-001",
  "payload":{"dispatched":[{"worker_id":"A"},{"worker_id":"B"}],
  "done_pattern":"worker.*.done","failed_pattern":"worker.*.failed",
  "timeout":"10m"}}' | kwike collect --state-dir ./state

# Record worker completions
echo '{"type":"worker.A.done","thread_id":"T1","id":"evt-002",
  "payload":{"result":"success"}}' | kwike collect --state-dir ./state

# Use as a consumer tool (in consumer.yaml)
tool:
  command: kwike
  args: ["collect", "--state-dir", "./state/collect", "--daemon", "local"]
  stdin: event
  timeout: 1m</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>--state-dir &lt;path&gt;</code> - Directory for state files (required)</li>
                    <li><code>--daemon &lt;url&gt;</code> - Daemon socket path (default: local)</li>
                </ul>
                <div class="command-note">
                    <strong>State Management:</strong> State is crash-safe and idempotent. Re-processing the same event is a no-op (deduped by ID). Completed collections are archived to <code>.completed/</code>.
                </div>
            </div>

            <div class="command-card">
                <h4><code>kwike consume generate</code></h4>
                <p class="command-desc">Scaffold consumer configurations with archetype patterns.</p>
                <div class="code-block">
<pre><code># Generate full agent directory
kwike consume generate ./agents implementer --types beads.task

# Single config file
kwike consume generate ./consumer.yaml --types beads.task

# With archetype pattern
kwike consume generate ./patterns/review --archetype review-loop --types task.implement

# Available archetypes:
# - linear: Fire-and-forget
# - self-spec: Contract in uniform
# - custom-schema: External JSON schema
# - review-loop: Bidirectional implementer/reviewer
# - delegation: Specialist routing
# - fan-out: Parallel workers with fan-in
# - multi-phase-addressing: Route to originating phase</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>&lt;path&gt;</code> - Output directory or file</li>
                    <li><code>&lt;name&gt;</code> - Consumer name (optional)</li>
                    <li><code>--types &lt;event-types&gt;</code> - Event types to consume</li>
                    <li><code>--archetype &lt;pattern&gt;</code> - Pattern archetype</li>
                    <li><code>--llm &lt;command&gt;</code> - LLM command (default: claude)</li>
                    <li><code>--daemon &lt;url&gt;</code> - Daemon URL</li>
                </ul>
            </div>
        </div>

        <div class="cli-category">
            <h3>Query & Management Commands</h3>
            <p class="category-intro">Inspect events, manage sessions, and maintain daemon state.</p>

            <div class="command-card">
                <h4><code>kwike events</code></h4>
                <p class="command-desc">Query events from the daemon event log.</p>
                <div class="code-block">
<pre><code># List all events
kwike events list

# Filter by type glob
kwike events list --type "beads.*"

# After cursor (pagination)
kwike events list --after &lt;event-id&gt;

# Show specific event
kwike events show &lt;event-id&gt;

# Acknowledge events (advance cursor)
kwike events ack --consumer &lt;name&gt; --through &lt;event-id&gt;</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>list</code> - List events with optional filters</li>
                    <li><code>show &lt;id&gt;</code> - Display full event details</li>
                    <li><code>ack</code> - Acknowledge events (advance consumer cursor)</li>
                    <li><code>--type &lt;glob&gt;</code> - Filter by event type pattern</li>
                    <li><code>--after &lt;id&gt;</code> - Start after specific event ID</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike sessions</code></h4>
                <p class="command-desc">Manage consumer session state and cleanup orphaned sessions.</p>
                <div class="code-block">
<pre><code># List sessions for a consumer
kwike sessions list --consumer implementer.yaml

# Cleanup stale sessions
kwike sessions cleanup --consumer implementer.yaml</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>list</code> - List active sessions</li>
                    <li><code>cleanup</code> - Remove orphaned session state</li>
                    <li><code>--consumer &lt;config&gt;</code> - Consumer config file</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike threads</code></h4>
                <p class="command-desc">List active event threads (events with unresolved descendants).</p>
                <div class="code-block">
<pre><code># All active threads
kwike threads list

# Filter by recency
kwike threads list --active-since 24h</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>list</code> - Display active threads</li>
                    <li><code>--active-since &lt;duration&gt;</code> - Filter by age</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike config</code></h4>
                <p class="command-desc">Validate configuration files and display current config.</p>
                <div class="code-block">
<pre><code># Validate consumer config
kwike config validate --config consumer.yaml

# Show effective configuration
kwike config show</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>validate</code> - Check config syntax and semantics</li>
                    <li><code>show</code> - Display merged config (file + defaults)</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike daemons</code></h4>
                <p class="command-desc">Manage running daemon processes.</p>
                <div class="code-block">
<pre><code># List running daemons
kwike daemons list

# Stop specific daemon
kwike daemons stop &lt;pid&gt;

# Stop all daemons
kwike daemons stop --all</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>list</code> - Show running daemons with PIDs</li>
                    <li><code>stop &lt;pid&gt;</code> - Gracefully stop daemon by PID</li>
                    <li><code>--all</code> - Stop all running daemons</li>
                </ul>
            </div>

            <div class="command-card">
                <h4><code>kwike memory</code></h4>
                <p class="command-desc">Persistent agent learnings store. Agents record conventions, error patterns, and gotchas that survive across sessions.</p>
                <div class="code-block">
<pre><code># Add a learning (via flag or stdin)
kwike memory add convention imports --content "Use absolute imports"
echo "Check response.ok" | kwike memory add error-pattern oauth

# List all learnings (or filter by category)
kwike memory list
kwike memory list --category convention

# Search across all fields (case-insensitive)
kwike memory search "oauth"

# State directory resolution
kwike memory list --state /path/to/state   # explicit
KWIKE_STATE_DIR=/path kwike memory list    # env var</code></pre>
                </div>
                <ul class="flag-list">
                    <li><code>add &lt;category&gt; &lt;key&gt;</code> - Add or upsert a learning</li>
                    <li><code>list</code> - List learnings (JSON-lines output)</li>
                    <li><code>search &lt;query&gt;</code> - Search by content, key, or category</li>
                    <li><code>--content &lt;text&gt;</code> - Learning content (or pipe via stdin)</li>
                    <li><code>--category &lt;name&gt;</code> - Filter by category</li>
                    <li><code>--state &lt;dir&gt;</code> - Override state directory</li>
                </ul>
                <div class="command-note">
                    <strong>Storage:</strong> Learnings persist as JSONL at <code>KWIKE_STATE_DIR/memory/learnings.jsonl</code> with file locking for concurrent access safety.
                </div>
            </div>

            <div class="command-card">
                <h4><code>kwike uniform</code></h4>
                <p class="command-desc">Inspect and document uniform template configuration including tool permissions.</p>
                <div class="code-block">
<pre><code># Show tool permissions for a consumer
kwike uniform tools --config consumer.yaml</code></pre>
                </div>
            </div>

            <div class="command-card">
                <h4><code>kwike doc</code></h4>
                <p class="command-desc">View embedded documentation on patterns and architecture.</p>
                <div class="code-block">
<pre><code># List available topics
kwike doc

# View specific topic
kwike doc daemon
kwike doc patterns
kwike doc memory</code></pre>
                </div>
            </div>

            <div class="command-card">
                <h4><code>kwike version</code></h4>
                <p class="command-desc">Display kwike version and build information.</p>
                <div class="code-block">
<pre><code>kwike version</code></pre>
                </div>
            </div>
        </div>

        <div class="cli-category">
            <h3>Environment Variables</h3>
            <p class="category-intro">
                When <code>kwike consume</code> spawns the LLM subprocess, it passes context via environment variables.
                Access these in your LLM command or bash wrapper scripts.
            </p>

            <div class="env-vars-grid">
                <div class="env-var-card">
                    <h4><code>KWIKE_PROMPT</code></h4>
                    <p>Rendered uniform template output. The complete prompt for the LLM.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_SCHEMA_SPEC</code></h4>
                    <p>Output contract specification when schema validation is enabled. Appended to prompt for structured output.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_EVENT_ID</code></h4>
                    <p>UUIDv7 identifier of the current event being processed.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_EVENT_TYPE</code></h4>
                    <p>Event type string (e.g., <code>beads.task</code>, <code>chain.middle</code>).</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_CONSUMER</code></h4>
                    <p>Consumer name from the config file. Identifies which consumer is processing.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_SESSION_ID</code></h4>
                    <p>Session ID when resume_flag is enabled. Allows LLM to maintain context across events.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_CONFIG_DIR</code></h4>
                    <p>Absolute path to the consumer config directory.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_STATE_DIR</code></h4>
                    <p>Absolute path to the state directory (session files, cursor state).</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_UNIFORM_DIR</code></h4>
                    <p>Absolute path to the uniform template directory.</p>
                </div>

                <div class="env-var-card">
                    <h4><code>KWIKE_WORKING_DIR</code></h4>
                    <p>Absolute path to git root (or current working directory if not in git).</p>
                </div>
            </div>

            <div class="env-usage-example">
                <h4>Usage Patterns</h4>
                <div class="code-block">
<pre><code># Via stdin (recommended for Claude CLI)
llm:
  command: claude
  stdin: [prompt, spec]  # KWIKE_PROMPT + KWIKE_SCHEMA_SPEC via stdin
  args: ["--print"]

# Via args (explicit interpolation)
llm:
  command: claude
  args:
    - "--"
    - "\${KWIKE_PROMPT}\${KWIKE_SCHEMA_SPEC}"

# In bash wrapper script
#!/usr/bin/env bash
set -euo pipefail

# All KWIKE_* vars available in environment
echo "Processing event: \${KWIKE_EVENT_ID}"
echo "Type: \${KWIKE_EVENT_TYPE}"

# Access prompt content
echo "\${KWIKE_PROMPT}" | jq -r '.description'</code></pre>
                </div>
            </div>
        </div>

        <div class="cli-category">
            <h3>Quick Reference</h3>
            <p class="category-intro">All commands grouped by purpose with one-line descriptions.</p>

            <table class="command-reference-table">
                <thead>
                    <tr>
                        <th>Command</th>
                        <th>Category</th>
                        <th>Description</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td><code>kwike daemon</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Event store server (Unix socket or HTTP)</td>
                    </tr>
                    <tr>
                        <td><code>kwike dispatch</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Emit JSON events from stdin</td>
                    </tr>
                    <tr>
                        <td><code>kwike watch</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Poll command and dispatch output periodically</td>
                    </tr>
                    <tr>
                        <td><code>kwike consume</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Process events and supervise LLM execution</td>
                    </tr>
                    <tr>
                        <td><code>kwike consume generate</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Scaffold consumer configs with archetypes</td>
                    </tr>
                    <tr>
                        <td><code>kwike collect</code></td>
                        <td class="category-badge pipeline">Pipeline</td>
                        <td>Track fan-out completions and emit collected event</td>
                    </tr>
                    <tr>
                        <td><code>kwike events list</code></td>
                        <td class="category-badge query">Query</td>
                        <td>List events with optional filters</td>
                    </tr>
                    <tr>
                        <td><code>kwike events show</code></td>
                        <td class="category-badge query">Query</td>
                        <td>Display full event details</td>
                    </tr>
                    <tr>
                        <td><code>kwike events ack</code></td>
                        <td class="category-badge query">Query</td>
                        <td>Acknowledge events (advance cursor)</td>
                    </tr>
                    <tr>
                        <td><code>kwike sessions list</code></td>
                        <td class="category-badge management">Management</td>
                        <td>List consumer session state</td>
                    </tr>
                    <tr>
                        <td><code>kwike sessions cleanup</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Remove orphaned sessions</td>
                    </tr>
                    <tr>
                        <td><code>kwike threads list</code></td>
                        <td class="category-badge query">Query</td>
                        <td>List active event threads</td>
                    </tr>
                    <tr>
                        <td><code>kwike config validate</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Validate configuration files</td>
                    </tr>
                    <tr>
                        <td><code>kwike config show</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Display effective configuration</td>
                    </tr>
                    <tr>
                        <td><code>kwike daemons list</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Show running daemon processes</td>
                    </tr>
                    <tr>
                        <td><code>kwike daemons stop</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Stop daemon by PID or all</td>
                    </tr>
                    <tr>
                        <td><code>kwike memory</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Persistent agent learnings (add, list, search)</td>
                    </tr>
                    <tr>
                        <td><code>kwike uniform tools</code></td>
                        <td class="category-badge management">Management</td>
                        <td>Inspect consumer tool permissions</td>
                    </tr>
                    <tr>
                        <td><code>kwike doc</code></td>
                        <td class="category-badge docs">Docs</td>
                        <td>View embedded documentation</td>
                    </tr>
                    <tr>
                        <td><code>kwike version</code></td>
                        <td class="category-badge docs">Docs</td>
                        <td>Show version and build info</td>
                    </tr>
                </tbody>
            </table>

            <div class="daemon-modes-callout">
                <h4>Daemon Modes</h4>
                <ul>
                    <li><strong>Unix Socket (default):</strong> <code>kwike daemon</code> - Best for local workflows, no network exposure</li>
                    <li><strong>HTTP:</strong> <code>kwike daemon --http :8080</code> - Remote access, mesh routing, multi-machine setups</li>
                    <li><strong>Mesh Routing:</strong> Configure <code>namespaces</code> and <code>upstreams</code> in kwike.yaml to route events across daemon boundaries</li>
                </ul>
            </div>
        </div>
    `;

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('cli-content');
        if (container) {
            container.innerHTML = cliContent;
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
