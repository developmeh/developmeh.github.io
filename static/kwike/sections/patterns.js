// Patterns Section Content
(function() {
    const patternsContent = `
        <div class="patterns-overview">
            <p class="section-intro">
                Real-world orchestration patterns from kwik-e-mart examples. These patterns demonstrate how to compose
                consumers, sessions, and events into production workflows.
            </p>
        </div>

        <div class="patterns-tabs">
            <button class="tab-button active" data-tab="review-loop">Review Loop</button>
            <button class="tab-button" data-tab="delegation">Delegation</button>
            <button class="tab-button" data-tab="fan-out">Fan-Out</button>
            <button class="tab-button" data-tab="mesh">Mesh</button>
            <button class="tab-button" data-tab="validation">Validation</button>
            <button class="tab-button" data-tab="recovery">Recovery</button>
        </div>

        <div class="tab-content" id="review-loop" style="display: block;">
            <div class="pattern-section">
                <h3>Review Loop Pattern</h3>
                <p>Bidirectional implementer/reviewer with session resumption for iterative code review.</p>

                <div class="pattern-card">
                    <h4>Event Flow</h4>
                    <div class="ascii-diagram">
<pre>
Task → Implementer (fresh session)
  ↓
  Implementer emits: task.implement.done
  ↓
  → Reviewer (fresh session)
    ↓
    Review fails → Reviewer emits: task.implement.done.failed (rejected)
    ↓
    → Implementer <strong>resumes SAME session</strong> with feedback
      ↓
      Implementer fixes issues → Emits: task.review.rejected.done
      ↓
      → Reviewer <strong>resumes SAME session</strong> to re-review
        ↓
        All issues fixed → Reviewer emits: task.review.rejected.done.done (approved)
</pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Implementer Configuration</h4>
                    <div class="code-block">
<pre><code>session:
  resume_types:
    - "task.review.rejected"  # Resume when reviewer rejects
  fresh_types:
    - "task.implement"        # Fresh start for new tasks
  timeout: 1h

llm:
  resume_flag: true  # Injects --resume \${KWIKE_SESSION_ID}</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Reviewer Configuration</h4>
                    <div class="code-block">
<pre><code>session:
  resume_types:
    - "task.review.rejected.done"  # Resume when checking fixes
  fresh_types:
    - "task.implement.done"        # Fresh start for new implementations
  timeout: 30m

llm:
  resume_flag: true</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>When to Use</h4>
                    <ul class="use-case-list">
                        <li><strong>Code review workflows</strong> - Automated agents review implementations</li>
                        <li><strong>Multi-stage approval</strong> - Implement → review → re-implement → approve</li>
                        <li><strong>Quality gates with iteration</strong> - Tests → fix → re-test cycles</li>
                        <li><strong>Collaborative agents</strong> - Workflows where feedback is expected</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="tab-content" id="delegation" style="display: none;">
            <div class="pattern-section">
                <h3>Delegation Pattern</h3>
                <p>Agent delegates sub-task to specialist within the same workflow. Implementer delegates research to faster model.</p>

                <div class="pattern-card">
                    <h4>Event Flow</h4>
                    <div class="ascii-diagram">
<pre>
Implementer receives task
  → Analyzes request, needs context
  → Delegates research to Researcher (same thread)
    ↓
    Researcher (Haiku): Glob/Grep/Read
    ← Researcher completes, replies
  → Implementer <strong>resumes with results</strong>
  → Uses findings to write code
</pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Implementer Configuration</h4>
                    <div class="code-block">
<pre><code>name: implementer
description: Sonnet-class code implementer

source:
  types: ["feature.implement"]

uniform:
  path: ./uniforms/implementer.md

llm:
  command: claude
  args: ["--model", "sonnet", ...]
  timeout: 15m  # Includes waiting for research</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Researcher Configuration</h4>
                    <div class="code-block">
<pre><code>name: researcher
description: Haiku-class code researcher

source:
  types: ["research.request"]

uniform:
  path: ./uniforms/researcher.md

llm:
  command: claude
  args: ["--model", "haiku", ...]
  timeout: 5m  # Research should be quick</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>When to Use</h4>
                    <ul class="use-case-list">
                        <li><strong>Model specialization</strong> - Fast model for search, powerful model for coding</li>
                        <li><strong>Separable research step</strong> - Context gathering clearly distinct from implementation</li>
                        <li><strong>Read-only exploration</strong> - Researcher can't accidentally modify code</li>
                        <li><strong>Time boundaries</strong> - Research gets 5m, implementation gets 15m</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="tab-content" id="fan-out" style="display: none;">
            <div class="pattern-section">
                <h3>Fan-Out Pattern</h3>
                <p>Orchestrator dispatches parallel workers. <code>kwike collect</code> tracks completions and emits a collected event when all workers report—no LLM needed for fan-in.</p>

                <div class="pattern-card">
                    <h4>Event Flow</h4>
                    <div class="ascii-diagram">
<pre>
Task → Orchestrator receives job.fanout
  → Dispatches Worker-A, Worker-B, Worker-C (parallel)
  → Emits job.fanout.dispatched (with worker list + patterns)
    ↓                ↓                ↓
    Worker-A       Worker-B         Worker-C
    completes      completes        completes
    ↓                ↓                ↓
  → <strong>kwike collect</strong> tracks each worker.*.done
  → State: 1/3 → 2/3 → 3/3
  → Emits job.fanout.collected (aggregated results)
    ↓
  → Orchestrator <strong>resumes</strong> with all results
</pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Orchestrator Configuration</h4>
                    <div class="code-block">
<pre><code>name: orchestrator
description: Dispatches parallel workers, resumes on collection

source:
  types:
    - "job.fanout"            # Fresh start — decompose and dispatch
    - "job.fanout.collected"  # Resume — all workers finished

session:
  fresh_types: ["job.fanout"]
  resume_types: ["job.fanout.collected"]
  timeout: 30m

llm:
  resume_flag: true
  command: claude
  args: ["--model", "sonnet", ...]</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Collector Configuration</h4>
                    <p>The collector is a pure-code consumer—no LLM needed. It uses <code>kwike collect</code> as a tool to track completions:</p>
                    <div class="code-block">
<pre><code>name: collector
description: Tracks fan-out completions via kwike collect

source:
  types:
    - "job.fanout.dispatched"  # Initialize collection
    - "worker.*.done"          # Record completions
    - "worker.*.failed"        # Record failures

tool:
  command: kwike
  args: ["collect", "--state-dir", "./state/collect", "--daemon", "local"]
  stdin: event
  timeout: 1m</code></pre>
                    </div>
                    <div class="command-note">
                        <strong>No LLM cost:</strong> The collector is deterministic code, not an LLM call. It tracks state on disk and emits the collected event when expected == done + failed.
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Worker Configuration</h4>
                    <div class="code-block">
<pre><code>name: worker
description: Parallel task processor

source:
  types: ["work.*"]

flow:
  max_concurrent: 3  # 3 workers in parallel

llm:
  command: claude
  args: ["--model", "haiku", ...]  # Fast execution</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>When to Use</h4>
                    <ul class="use-case-list">
                        <li><strong>Parallel decomposition</strong> - Task splits into independent subtasks</li>
                        <li><strong>Multi-source aggregation</strong> - Collect data from multiple sources concurrently</li>
                        <li><strong>Pipeline stages</strong> - Each worker handles different processing stage</li>
                        <li><strong>Map-reduce</strong> - Fan out map phase, fan in reduce phase via collector</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="tab-content" id="mesh" style="display: none;">
            <div class="pattern-section">
                <h3>Mesh Networking</h3>
                <p>Route events across daemon boundaries with namespace-based routing, upstream forwarding, and mTLS security.</p>

                <div class="pattern-card">
                    <h4>Network Topology</h4>
                    <div class="ascii-diagram">
<pre>
┌─────────────────────────────────────────────────────────────┐
│                    Production Network                        │
│  ┌─────────────┐      mTLS      ┌─────────────┐            │
│  │  daemon-a   │ ←────────────→ │  daemon-b   │            │
│  │ (beads.*)   │                │ (ci.*)      │            │
│  └──────┬──────┘                └──────┬──────┘            │
│         │                              │                    │
│    consumers                      consumers                 │
└─────────────────────────────────────────────────────────────┘
            ↑ upstream                    ↑ upstream
            │ (broadcast)                 │ (broadcast)
┌───────────┴─────────────────────────────┴───────────────────┐
│                    Developer Machines                        │
│  ┌─────────────┐                ┌─────────────┐            │
│  │  local-dev  │                │  local-dev  │            │
│  │  daemon     │                │  daemon     │            │
│  └─────────────┘                └─────────────┘            │
└─────────────────────────────────────────────────────────────┘
</pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Upstream Configuration (kwike.yaml)</h4>
                    <p>Define upstream daemons to forward events based on namespace patterns:</p>
                    <div class="code-block">
<pre><code># Local daemon configuration
daemon:
  socket: ./kwike.sock
  http: :8080

# Upstream routing - forward matching events
upstreams:
  - name: production
    url: https://prod.example.com:8443
    tls:
      cert: /etc/kwike/client.crt
      key: /etc/kwike/client.key
      ca: /etc/kwike/ca.crt
    namespaces:
      - "beads.*"      # Forward all beads events
      - "ci.build.*"   # Forward CI build events

  - name: monitoring
    url: https://mon.example.com:8443
    tls:
      cert: /etc/kwike/client.crt
      key: /etc/kwike/client.key
      ca: /etc/kwike/ca.crt
    namespaces:
      - "*.done"       # Forward all completion events
      - "*.failed"     # Forward all failure events</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Broadcast Dispatch</h4>
                    <p>Send events to multiple daemons simultaneously for fan-out across networks:</p>
                    <div class="code-block">
<pre><code># Dispatch to local daemon (default)
echo '{"task": "build"}' | kwike dispatch --type ci.build

# Dispatch to specific upstream
echo '{"task": "build"}' | kwike dispatch --type ci.build \\
    --daemon https://prod.example.com:8443

# Broadcast to all upstreams matching namespace
echo '{"task": "build"}' | kwike dispatch --type ci.build \\
    --broadcast

# Event routing flow:
# 1. Local daemon receives event
# 2. Checks namespace against upstream patterns
# 3. Forwards to matching upstreams via mTLS
# 4. Each upstream stores + routes to its consumers</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>mTLS Configuration</h4>
                    <p>Mutual TLS ensures both client and server authenticate:</p>
                    <div class="code-block">
<pre><code># Server-side (daemon receiving events)
daemon:
  http: :8443
  tls:
    cert: /etc/kwike/server.crt
    key: /etc/kwike/server.key
    client_ca: /etc/kwike/ca.crt  # Verify client certs
    require_client_cert: true

# Client-side (daemon forwarding events)
upstreams:
  - name: production
    url: https://prod.example.com:8443
    tls:
      cert: /etc/kwike/client.crt  # Present to server
      key: /etc/kwike/client.key
      ca: /etc/kwike/ca.crt        # Verify server cert</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Use Cases</h4>
                    <ul class="use-case-list">
                        <li><strong>Multi-region deployment</strong> - Route events to regional daemons based on namespace</li>
                        <li><strong>Dev/Prod separation</strong> - Local dev dispatches to prod for CI/CD triggers</li>
                        <li><strong>Event aggregation</strong> - Central monitoring daemon receives *.done from all environments</li>
                        <li><strong>Air-gapped networks</strong> - Secure event routing across network boundaries with mTLS</li>
                        <li><strong>Hybrid cloud</strong> - On-prem daemons forward to cloud, or vice versa</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="tab-content" id="validation" style="display: none;">
            <div class="pattern-section">
                <h3>Validation Patterns</h3>
                <p>Two approaches to ensuring LLM output contracts: embedded vs external schemas.</p>

                <div class="comparison-table">
                    <table>
                        <thead>
                            <tr>
                                <th>Aspect</th>
                                <th>self-spec</th>
                                <th>custom-schema</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td><strong>Contract location</strong></td>
                                <td>In uniform.md</td>
                                <td>In schema.json</td>
                            </tr>
                            <tr>
                                <td><strong>Validation</strong></td>
                                <td>Manual (your code)</td>
                                <td>Automatic (kwike)</td>
                            </tr>
                            <tr>
                                <td><strong>Config</strong></td>
                                <td><code>schema: none</code></td>
                                <td><code>schema: ./schema.json</code></td>
                            </tr>
                            <tr>
                                <td><strong>Stdin</strong></td>
                                <td><code>stdin: [prompt]</code></td>
                                <td><code>stdin: [prompt, spec]</code></td>
                            </tr>
                            <tr>
                                <td><strong>Retries</strong></td>
                                <td>No auto-retry</td>
                                <td>Auto-retry on validation failure</td>
                            </tr>
                            <tr>
                                <td><strong>Use case</strong></td>
                                <td>Custom formats, prototyping</td>
                                <td>Standard JSON validation</td>
                            </tr>
                            <tr>
                                <td><strong>Reusability</strong></td>
                                <td>Template-specific</td>
                                <td>Reusable across agents</td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div class="pattern-card">
                    <h4>self-spec: Contract in Uniform</h4>
                    <div class="code-block">
<pre><code>## Output Contract (MANDATORY)

Your response MUST end with a JSON code block:

\`\`\`json
{
  "status": "done|failed|blocked",
  "confidence": 0.0-1.0,
  "analysis": "Your detailed analysis"
}
\`\`\`

Consumer YAML:
llm:
  stdin: [prompt]  # Only prompt
  schema: none     # No validation</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>custom-schema: External JSON Schema</h4>
                    <div class="code-block">
<pre><code>// schema.json
{
  "$id": "kwike.contract.example",
  "required": ["status", "result"],
  "properties": {
    "status": {"enum": ["done", "failed", "blocked"]},
    "result": {
      "properties": {
        "processed": {"type": "boolean"}
      }
    }
  }
}

Consumer YAML:
llm:
  stdin: [prompt, spec]  # Inject schema spec
  schema: ./schema.json
  schema_max_retries: 3</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>When to Use Each</h4>
                    <ul class="use-case-list">
                        <li><strong>Use self-spec for:</strong> Rapid prototyping, custom formats, single-agent scenarios</li>
                        <li><strong>Use custom-schema for:</strong> Multiple agents sharing contract, automatic validation, production reliability</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="tab-content" id="recovery" style="display: none;">
            <div class="pattern-section">
                <h3>Recovery Patterns</h3>
                <p>Three patterns for handling failures and routing: linear, fresh-start, and multi-phase-addressing.</p>

                <div class="pattern-card">
                    <h4>Linear: Fire-and-Forget</h4>
                    <p>Simplest pipeline with no retries, sessions, or threading.</p>
                    <div class="ascii-diagram">
<pre>
Event → Consumer → LLM (one-shot)
                → Done (no reply events)

Config:
  max_retries: 0
  emit_done: false
  resume_flag: false
</pre>
                    </div>
                    <p><strong>Use for:</strong> Simple one-off tasks, prototyping, logging, notifications</p>
                </div>

                <div class="pattern-card">
                    <h4>Fresh-Start: Loop Recovery</h4>
                    <p>Break stuck retry loops by emitting new events with accumulated context.</p>
                    <div class="ascii-diagram">
<pre>
task.process (retry 1) → FAILED
task.process (retry 2) → FAILED (resume same session)
task.process (retry 3) → FAILED (max_retries)
  ↓
Orchestrator detects stuck state
  ↓
Emits <strong>NEW event</strong>: task.clarified
  + enriched context (errors, history)
  + NEW event_id → NEW session_id
  ↓
Consumer starts fresh with full context → SUCCESS
</pre>
                    </div>
                    <p><strong>Use for:</strong> Stuck agent loops, ambiguous requirements, escalation to human-assisted recovery</p>
                </div>

                <div class="pattern-card">
                    <h4>Multi-Phase-Addressing: Event Routing</h4>
                    <p>Route lifecycle events back to originating phase using <code>addresses_field</code>.</p>
                    <div class="ascii-diagram">
<pre>
build-latest fails → fix → review → ??? → build-latest resumes
mirror-it-env fails → fix → review → ??? → mirror-it-env resumes

Solution: addresses_field routing

Phase agent emits:
  payload: { originating_consumer: "phase-agent-1" }

Reviewer outputs:
  { "target_agent": "phase-agent-1" }

Reviewer config:
  addresses_field: target_agent  # Extract → event.addresses

Result:
  workflow.fix.review.done
    addresses: ["phase-agent-1"]  ← Only this phase processes
</pre>
                    </div>
                    <p><strong>Use for:</strong> Multi-phase pipelines where any phase can fail, shared fix/review workflow, correct phase resumption</p>
                </div>

                <div class="pattern-card">
                    <h4>When to Use Each</h4>
                    <ul class="use-case-list">
                        <li><strong>Linear:</strong> Prototyping, simple tasks, learning kwike fundamentals</li>
                        <li><strong>Fresh-Start:</strong> Agent stuck in retry loop, need different approach with accumulated context</li>
                        <li><strong>Multi-Phase-Addressing:</strong> Multiple phase agents sharing implementer/reviewer, need precise routing</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="key-benefits">
            <h3>Pattern Benefits</h3>
            <div class="benefits-grid">
                <div class="benefit-card">
                    <strong>Composable by Design</strong>
                    <p>Mix and match patterns. Review-loop + delegation + fan-out = complex workflows from simple primitives.</p>
                </div>
                <div class="benefit-card">
                    <strong>Observable & Debuggable</strong>
                    <p>All events in events.jsonl. Session transcripts show full history. Trace causality through event chains.</p>
                </div>
                <div class="benefit-card">
                    <strong>Crash-Only Recovery</strong>
                    <p>Don't mutate state, emit events. Fresh starts with context beats session patching.</p>
                </div>
                <div class="benefit-card">
                    <strong>Production-Ready</strong>
                    <p>Real patterns from kwik-e-mart. Tested in sr.ht orchestration and beads workflows.</p>
                </div>
            </div>
        </div>
    `;

    // Tab switching logic
    const setupTabs = () => {
        const tabButtons = document.querySelectorAll('#patterns-content .tab-button');
        const tabContents = document.querySelectorAll('#patterns-content .tab-content');

        tabButtons.forEach(button => {
            button.addEventListener('click', () => {
                // Remove active from all buttons and hide all contents
                tabButtons.forEach(btn => btn.classList.remove('active'));
                tabContents.forEach(content => content.style.display = 'none');

                // Activate clicked tab
                button.classList.add('active');
                const tabId = button.getAttribute('data-tab');
                const targetTab = document.getElementById(tabId);
                if (targetTab) {
                    targetTab.style.display = 'block';
                }
            });
        });
    };

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('patterns-content');
        if (container) {
            container.innerHTML = patternsContent;
            setupTabs();
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
