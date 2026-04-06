// Consumer Patterns Section Content
(function() {
    const consumersContent = `
        <div class="consumers-overview">
            <p class="section-intro">
                Consumers can integrate with Claude CLI directly or use bash script wrappers for custom workflows.
                Both patterns leverage the same event-driven foundation with uniforms (prompt templates) and result validation.
            </p>
        </div>

        <div class="patterns-tabs">
            <button class="tab-button active" data-tab="claude-cli">Claude CLI Integration</button>
            <button class="tab-button" data-tab="bash-wrapper">Bash Script Wrappers</button>
        </div>

        <div class="tab-content" id="claude-cli" style="display: block;">
            <div class="pattern-section">
                <h3>Direct Claude CLI Integration</h3>
                <p>The most common pattern: supervise Claude CLI as a subprocess with controlled permissions.</p>

                <div class="pattern-card">
                    <h4>Consumer Configuration (consumer.yaml)</h4>
                    <div class="code-block">
<pre><code>name: implementer
description: Sonnet-class code implementer

source:
  daemon: local
  types:
    - "beads.task"
    - "beads.feature"

uniform:
  path: ./uniform.md

tool:
  command: claude
  args:
    - "--model"
    - "sonnet"
    - "--permission-mode"
    - "dontAsk"
    - "--allowedTools"
    - "Read"
    - "Edit"
    - "Bash(git add *)"
    - "Bash(git commit *)"
    - "Bash(kwike *)"
    - "--add-dir"
    - "../../.."
    - "--"
    - "\${KWIKE_PROMPT}\${KWIKE_SCHEMA_SPEC}"
  timeout: 10m
  resume_flag: true
  schema_max_retries: 3</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Uniform Template (uniform.md)</h4>
                    <p>Templates render event data into focused prompts using Go text/template syntax:</p>
                    <div class="code-block">
<pre><code># Implementer Agent

You are an implementation agent for ticket {{ .Event.Payload.ticket_id }}.

## Task

{{ .Event.Payload.description }}

## Instructions

1. Read the task breakdown
2. Implement each task in a sub-branch
3. Commit your changes

## OUTPUT CONTRACT (MANDATORY)

Return JSON with:
- status: "done|blocked|error"
- summary: "what was implemented"
- tasks_implemented: N</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Result Validation</h4>
                    <p>Consumers validate LLM output against JSON schemas. On failure, retry with error feedback:</p>
                    <ul class="feature-list">
                        <li><strong>Schema validation</strong> - Enforce required fields and types</li>
                        <li><strong>Auto-retry</strong> - Failures trigger retry with validation errors</li>
                        <li><strong>Max retries</strong> - Configurable retry limit (default: 3)</li>
                        <li><strong>Dead letter</strong> - Persistent failures become dead_letter events</li>
                    </ul>
                </div>

                <div class="flow-diagram">
                    <div class="flow-step-vertical">
                        <div class="flow-node">Poll Event</div>
                        <div class="arrow-down">↓</div>
                    </div>
                    <div class="flow-step-vertical">
                        <div class="flow-node">Render Uniform</div>
                        <div class="arrow-down">↓</div>
                    </div>
                    <div class="flow-step-vertical">
                        <div class="flow-node">Invoke Claude CLI</div>
                        <div class="arrow-down">↓</div>
                    </div>
                    <div class="flow-step-vertical">
                        <div class="flow-node">Validate Result</div>
                        <div class="arrow-down">↓</div>
                    </div>
                    <div class="flow-decision">
                        <div class="flow-node decision">Valid?</div>
                        <div class="decision-branches">
                            <div class="branch-yes">
                                <span class="arrow-label">Yes</span>
                                <div class="arrow-down">↓</div>
                                <div class="flow-node success">ACK + Dispatch Result</div>
                            </div>
                            <div class="branch-no">
                                <span class="arrow-label">No</span>
                                <div class="arrow-down">↓</div>
                                <div class="flow-node retry">Retry (max 3x)</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="tab-content" id="bash-wrapper" style="display: none;">
            <div class="pattern-section">
                <h3>Bash Script Wrapper Pattern</h3>
                <p>For custom workflows beyond LLM integration—chain events, coordinate external systems, or implement pure-bash logic.</p>

                <div class="pattern-card">
                    <h4>Consumer Config (Bash Handler)</h4>
                    <div class="code-block">
<pre><code>name: chain-handler-a
description: First stage in event chain

source:
  daemon: local
  types:
    - "chain.start"

uniform:
  path: ./uniform.md

llm:
  command: ./handler.sh
  timeout: 1m</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Bash Handler Script</h4>
                    <div class="code-block">
<pre><code>#!/usr/bin/env bash
set -euo pipefail

# Environment variables provided by kwike:
# - KWIKE_EVENT_ID
# - KWIKE_PROMPT (rendered uniform)
# - KWIKE_SESSION_ID

OUTPUT="\${CHAIN_OUTPUT:-./output.txt}"

# Do work
echo "Line 1: Started by daemon-a" >> "\${OUTPUT}"

# Dispatch next event in chain
kwike dispatch \\
    --type chain.middle \\
    --daemon "unix://\${DAEMON_SOCK}" \\
    <<< '{"step": 2, "origin": "daemon-a"}'

# Return structured result (validated by consumer)
cat <<EOF
{
  "status": "done",
  "summary": "Chain started, dispatched middle event"
}
EOF</code></pre>
                    </div>
                </div>

                <div class="pattern-card">
                    <h4>Use Cases</h4>
                    <ul class="use-case-list">
                        <li>
                            <strong>Event Chains</strong>
                            <p>Pass work between consumers via dispatch. Each stage does one thing, dispatches next.</p>
                        </li>
                        <li>
                            <strong>External System Integration</strong>
                            <p>React to events by calling APIs, updating databases, triggering CI/CD.</p>
                        </li>
                        <li>
                            <strong>Custom Validation</strong>
                            <p>Run tests, verify builds, check constraints before ACKing events.</p>
                        </li>
                        <li>
                            <strong>Mesh Routing</strong>
                            <p>Forward events across daemon boundaries for distributed workflows.</p>
                        </li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="key-benefits">
            <h3>Why This Works</h3>
            <div class="benefits-grid">
                <div class="benefit-card">
                    <strong>Crash-Only Design</strong>
                    <p>Each consumer invocation is fresh. No accumulated context. No conversation drift.</p>
                </div>
                <div class="benefit-card">
                    <strong>Deterministic Validation</strong>
                    <p>Code validates LLM output. Failures retry with feedback. Works after one attempt.</p>
                </div>
                <div class="benefit-card">
                    <strong>Composable by Default</strong>
                    <p>Consumers chain via events. Add new consumers without modifying existing ones.</p>
                </div>
                <div class="benefit-card">
                    <strong>Observable & Debuggable</strong>
                    <p>All events in events.jsonl. Replay workflows. Inspect state. Trace causality.</p>
                </div>
            </div>
        </div>
    `;

    // Tab switching logic
    const setupTabs = () => {
        const container = document.getElementById('consumers-content');
        const tabButtons = container.querySelectorAll('.tab-button');
        const tabContents = container.querySelectorAll('.tab-content');

        tabButtons.forEach(button => {
            button.addEventListener('click', () => {
                // Remove active from all buttons and hide all contents
                tabButtons.forEach(btn => btn.classList.remove('active'));
                tabContents.forEach(content => content.style.display = 'none');

                // Activate clicked tab
                button.classList.add('active');
                const tabId = button.getAttribute('data-tab');
                document.getElementById(tabId).style.display = 'block';
            });
        });
    };

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('consumers-content');
        if (container) {
            container.innerHTML = consumersContent;
            setupTabs();
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
