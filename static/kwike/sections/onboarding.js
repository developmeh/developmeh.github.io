// LLM Onboarding Section Content
(function() {
    const onboardingContent = `
        <div class="onboarding-intro">
            <p>Kwike's help structure is designed to enable LLMs to quickly understand commands and concepts, optimized for machine consumption while remaining human-readable.</p>
        </div>

        <div class="help-hierarchy">
            <h3>Layered Help System</h3>
            <p>Kwike provides help at multiple levels of detail, allowing both quick reference and deep understanding:</p>

            <div class="help-layers">
                <div class="help-layer" data-layer="quick">
                    <div class="layer-icon">
                        <svg viewBox="0 0 100 100" class="icon-svg">
                            <rect x="30" y="30" width="40" height="10" fill="currentColor" opacity="0.8"/>
                            <rect x="30" y="45" width="35" height="10" fill="currentColor" opacity="0.6"/>
                            <rect x="30" y="60" width="30" height="10" fill="currentColor" opacity="0.4"/>
                        </svg>
                    </div>
                    <h4>Quick Help</h4>
                    <code>kwike --help</code>
                    <p>Brief command-line flags and subcommand summary. Fast reference for agents that know the basics.</p>
                </div>

                <div class="help-layer" data-layer="embedded">
                    <div class="layer-icon">
                        <svg viewBox="0 0 100 100" class="icon-svg">
                            <rect x="25" y="25" width="50" height="50" fill="none" stroke="currentColor" stroke-width="3"/>
                            <path d="M 35 40 L 65 40 M 35 50 L 65 50 M 35 60 L 50 60" stroke="currentColor" stroke-width="2"/>
                        </svg>
                    </div>
                    <h4>Embedded Docs</h4>
                    <code>kwike doc</code>
                    <p>Detailed usage examples embedded in the binary. Shows typical workflows without leaving the terminal.</p>
                </div>

                <div class="help-layer" data-layer="manual">
                    <div class="layer-icon">
                        <svg viewBox="0 0 100 100" class="icon-svg">
                            <path d="M 30 20 L 50 20 L 70 35 L 70 80 L 30 80 Z" fill="none" stroke="currentColor" stroke-width="3"/>
                            <path d="M 50 20 L 50 35 L 70 35" fill="none" stroke="currentColor" stroke-width="3"/>
                            <path d="M 40 50 L 60 50 M 40 60 L 60 60 M 40 70 L 55 70" stroke="currentColor" stroke-width="2"/>
                        </svg>
                    </div>
                    <h4>Deep Dive Topics</h4>
                    <code>kwike doc &lt;topic&gt;</code>
                    <p>Comprehensive documentation on specific topics: consume, patterns, uniform, daemon. Each topic includes examples and configuration reference.</p>
                </div>
            </div>

            <div class="hierarchy-diagram">
                <svg viewBox="0 0 400 200" class="flow-svg">
                    <!-- Quick Help -->
                    <rect x="20" y="80" width="80" height="40" rx="5" fill="#2d3748" stroke="#4a5568" stroke-width="2"/>
                    <text x="60" y="105" text-anchor="middle" fill="#e2e8f0" font-size="12">kwike --help</text>

                    <!-- Arrow -->
                    <path d="M 100 100 L 140 100" stroke="#4a5568" stroke-width="2" marker-end="url(#arrow-help)"/>

                    <!-- Embedded Docs -->
                    <rect x="140" y="80" width="80" height="40" rx="5" fill="#2d3748" stroke="#4a5568" stroke-width="2"/>
                    <text x="180" y="105" text-anchor="middle" fill="#e2e8f0" font-size="12">kwike doc</text>

                    <!-- Arrow -->
                    <path d="M 220 100 L 260 100" stroke="#4a5568" stroke-width="2" marker-end="url(#arrow-help)"/>

                    <!-- Deep Dive -->
                    <rect x="260" y="80" width="120" height="40" rx="5" fill="#2d3748" stroke="#4a5568" stroke-width="2"/>
                    <text x="320" y="105" text-anchor="middle" fill="#e2e8f0" font-size="12">kwike doc &lt;topic&gt;</text>

                    <!-- Labels -->
                    <text x="120" y="75" text-anchor="middle" fill="#a0aec0" font-size="10">More Detail →</text>
                    <text x="60" y="145" text-anchor="middle" fill="#a0aec0" font-size="10">Fast</text>
                    <text x="180" y="145" text-anchor="middle" fill="#a0aec0" font-size="10">Contextual</text>
                    <text x="320" y="145" text-anchor="middle" fill="#a0aec0" font-size="10">Comprehensive</text>

                    <defs>
                        <marker id="arrow-help" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
                            <polygon points="0 0, 10 3, 0 6" fill="#4a5568" />
                        </marker>
                    </defs>
                </svg>
            </div>

            <div class="help-insight">
                <p><strong>Key Insight:</strong> All documentation is embedded in the binary. LLMs scan <code>kwike --help</code> for commands, browse topics with <code>kwike doc</code>, then deep-dive with <code>kwike doc &lt;topic&gt;</code>. No external files needed.</p>
            </div>

            <div class="error-docs-pointer">
                <h4>Contextual Errors Point to Documentation</h4>
                <p>Kwike errors are designed for LLM consumption—they show exactly what went wrong, what was expected, and where to find help:</p>
                <div class="terminal-output">
                    <pre><code>❌ Schema validation failed for event 019abc...

  Consumer: implementer
  Event:    beads.task

  Expected: { "status": "done|blocked", "summary": string }
  Got:      { "status": "done" }

  Missing required field: "summary"

  See: kwike doc uniform    (output contracts)
  See: kwike doc patterns   (self-spec examples)</code></pre>
                </div>
                <div class="terminal-output">
                    <pre><code>❌ Configuration error in ./agents/reviewer/consumer.yaml

  Line 12: source.types[0]
  Value:   "beads.*"

  Glob patterns not allowed in consumer types.
  Use exact event types: "beads.task.review"

  See: kwike doc consume    (source configuration)</code></pre>
                </div>
                <p class="error-insight">Contextual errors close the learning loop—the LLM sees what failed, why, and where to learn more. No guessing required.</p>
            </div>
        </div>

        <div class="uniform-contracts">
            <h3>Uniform Templates as Agent Contracts</h3>
            <p>Uniforms are markdown templates that define the exact behavior expected from an LLM agent. They act as executable contracts—the LLM receives a rendered uniform via stdin and must produce output matching the specified format.</p>

            <div class="contract-flow">
                <div class="flow-step">
                    <div class="step-number">1</div>
                    <h4>Template Definition</h4>
                    <p>Define agent behavior in a uniform template using Go template syntax:</p>
                    <pre><code>You are an implementing agent working on bead {{.ItemID}}.

## Task
**{{.Title}}**

## Instructions
1. Read the codebase
2. Implement the task
3. Commit your changes

## Response Format (CRITICAL)
Your final response MUST be exactly one of:
- \`DONE: &lt;one-line summary&gt;\`
- \`BLOCKED: &lt;reason&gt;\`</code></pre>
                </div>

                <div class="flow-step">
                    <div class="step-number">2</div>
                    <h4>Event Data Injection</h4>
                    <p>Kwike renders the template with event data available via <code>.Event.*</code> and consumer config via <code>.Consumer.*</code>:</p>
                    <pre><code>{{.Event.title}}      → "Fix authentication bug"
{{.Event.id}}         → "issue-42"
{{.Consumer.name}}    → "implementer"</code></pre>
                </div>

                <div class="flow-step">
                    <div class="step-number">3</div>
                    <h4>LLM Subprocess Execution</h4>
                    <p>The rendered uniform is sent to the LLM subprocess via stdin:</p>
                    <pre><code>kwike consume → render uniform → claude --print</code></pre>
                    <p>The LLM reads the contract, does the work, and produces structured output.</p>
                </div>
            </div>

            <div class="contract-example">
                <h4>Example: Implementer Uniform</h4>
                <div class="example-columns">
                    <div class="example-col">
                        <h5>Template (uniforms/implementer.md)</h5>
                        <pre><code>You are implementing {{.Event.title}}

## Description
{{.Event.description}}

## Instructions
- Implement the feature
- Write tests
- Commit changes

## Output Contract
Return JSON:
{
  "status": "done|blocked",
  "summary": "what you did"
}</code></pre>
                    </div>
                    <div class="example-col">
                        <h5>Rendered Prompt</h5>
                        <pre><code>You are implementing Add login button

## Description
Add a logout button to the nav bar

## Instructions
- Implement the feature
- Write tests
- Commit changes

## Output Contract
Return JSON:
{
  "status": "done|blocked",
  "summary": "what you did"
}</code></pre>
                    </div>
                </div>
            </div>

            <div class="uniform-insight">
                <p><strong>Philosophy:</strong> Uniforms eliminate ambiguity. The LLM doesn't guess what to do—it receives explicit instructions, context, and output format requirements. This crash-only, contract-first approach ensures reproducible agent behavior.</p>
            </div>
        </div>

        <div class="schema-validation">
            <h3>Schema Validation & Self-Correction Loop</h3>
            <p>Kwike enforces output contracts through JSON schema validation, enabling LLMs to self-correct via a feedback loop.</p>

            <div class="env-vars-grid">
                <div class="env-var-card">
                    <h4>KWIKE_PROMPT</h4>
                    <p>The rendered uniform template with all event data injected. Contains instructions, context, and the task description.</p>
                    <pre><code>export KWIKE_PROMPT="You are implementing task-42..."</code></pre>
                </div>

                <div class="env-var-card">
                    <h4>KWIKE_SCHEMA_SPEC</h4>
                    <p>The JSON schema specification that defines the required output structure. Automatically appended when <code>schema: self-spec</code> or <code>schema: &lt;path&gt;</code> is configured.</p>
                    <pre><code>## OUTPUT CONTRACT (MANDATORY)
Your response MUST end with JSON:
{
  "status": "done|blocked",
  "summary": "...",
  "tasks_completed": 0
}</code></pre>
                </div>
            </div>

            <div class="validation-loop-diagram">
                <h4>Validation & Retry Feedback Loop</h4>
                <svg viewBox="0 0 600 400" class="loop-svg">
                    <!-- LLM Subprocess -->
                    <rect x="50" y="50" width="150" height="60" rx="5" fill="#2d3748" stroke="#4a5568" stroke-width="2"/>
                    <text x="125" y="75" text-anchor="middle" fill="#e2e8f0" font-size="14" font-weight="bold">LLM Subprocess</text>
                    <text x="125" y="95" text-anchor="middle" fill="#a0aec0" font-size="11">claude --print</text>

                    <!-- Arrow to Output (produces) -->
                    <path d="M 200 80 L 240 80" stroke="#48bb78" stroke-width="3"/>
                    <polygon points="250,80 240,75 240,85" fill="#48bb78"/>
                    <text x="225" y="70" text-anchor="middle" fill="#48bb78" font-size="10">produces</text>

                    <!-- Output -->
                    <rect x="250" y="50" width="150" height="60" rx="5" fill="#2d3748" stroke="#48bb78" stroke-width="2"/>
                    <text x="325" y="75" text-anchor="middle" fill="#e2e8f0" font-size="14" font-weight="bold">JSON Output</text>
                    <text x="325" y="95" text-anchor="middle" fill="#a0aec0" font-size="10">{"status": "done"}</text>

                    <!-- Arrow to Validator -->
                    <path d="M 325 110 L 325 150" stroke="#4a5568" stroke-width="3"/>
                    <polygon points="325,160 320,150 330,150" fill="#4a5568"/>

                    <!-- Schema Validator -->
                    <rect x="250" y="160" width="150" height="60" rx="5" fill="#2d3748" stroke="#805ad5" stroke-width="2"/>
                    <text x="325" y="185" text-anchor="middle" fill="#e2e8f0" font-size="14" font-weight="bold">Schema Validator</text>
                    <text x="325" y="205" text-anchor="middle" fill="#a0aec0" font-size="10">kwike validate</text>

                    <!-- Success Path -->
                    <path d="M 400 190 L 470 190" stroke="#48bb78" stroke-width="3"/>
                    <polygon points="480,190 470,185 470,195" fill="#48bb78"/>
                    <text x="440" y="180" text-anchor="middle" fill="#48bb78" font-size="10">valid ✓</text>

                    <!-- Success Box -->
                    <rect x="480" y="160" width="100" height="60" rx="5" fill="#2d3748" stroke="#48bb78" stroke-width="2"/>
                    <text x="530" y="185" text-anchor="middle" fill="#48bb78" font-size="14" font-weight="bold">ACK Event</text>
                    <text x="530" y="205" text-anchor="middle" fill="#a0aec0" font-size="10">Task done</text>

                    <!-- Failure Path -->
                    <path d="M 325 220 L 325 270" stroke="#f56565" stroke-width="3"/>
                    <polygon points="325,280 320,270 330,270" fill="#f56565"/>
                    <text x="360" y="250" text-anchor="middle" fill="#f56565" font-size="10">invalid ✗</text>

                    <!-- Error Box -->
                    <rect x="250" y="280" width="150" height="60" rx="5" fill="#2d3748" stroke="#f56565" stroke-width="2"/>
                    <text x="325" y="305" text-anchor="middle" fill="#f56565" font-size="14" font-weight="bold">Validation Error</text>
                    <text x="325" y="325" text-anchor="middle" fill="#a0aec0" font-size="10">Missing "summary"</text>

                    <!-- Retry Path - loops around left side back to LLM -->
                    <path d="M 250 310 L 20 310 L 20 80 L 40 80" fill="none" stroke="#ed8936" stroke-width="3" stroke-dasharray="5,5"/>
                    <polygon points="50,80 40,75 40,85" fill="#ed8936"/>
                    <text x="20" y="200" text-anchor="start" fill="#ed8936" font-size="10" transform="rotate(-90 20 200)">retry with error</text>
                </svg>
            </div>

            <div class="validation-steps">
                <div class="validation-step">
                    <span class="step-icon">1</span>
                    <div class="step-content">
                        <h5>LLM Receives Contract</h5>
                        <p>The LLM subprocess receives <code>KWIKE_PROMPT</code> (instructions + context) and <code>KWIKE_SCHEMA_SPEC</code> (output requirements) via stdin or args.</p>
                    </div>
                </div>

                <div class="validation-step">
                    <span class="step-icon">2</span>
                    <div class="step-content">
                        <h5>Produces Structured Output</h5>
                        <p>The LLM attempts to produce JSON output matching the schema specification.</p>
                    </div>
                </div>

                <div class="validation-step">
                    <span class="step-icon">3</span>
                    <div class="step-content">
                        <h5>Schema Validation</h5>
                        <p>Kwike validates the output against the JSON schema. If valid, the event is ACKed. If invalid, kwike captures the validation error.</p>
                    </div>
                </div>

                <div class="validation-step">
                    <span class="step-icon">4</span>
                    <div class="step-content">
                        <h5>Retry with Error Context</h5>
                        <p>On validation failure, kwike retries the LLM with the original prompt PLUS the validation error message. The LLM self-corrects based on the specific error.</p>
                    </div>
                </div>
            </div>

            <div class="schema-example">
                <h4>Example: Self-Correction in Action</h4>
                <div class="correction-flow">
                    <div class="attempt">
                        <h5>Attempt 1 (Invalid)</h5>
                        <pre><code>{
  "status": "done"
  // Missing required "summary" field
}</code></pre>
                        <p class="error-msg">❌ Validation error: Missing required property: summary</p>
                    </div>

                    <div class="arrow-right">→</div>

                    <div class="attempt">
                        <h5>Attempt 2 (Valid)</h5>
                        <pre><code>{
  "status": "done",
  "summary": "Added logout button to nav"
}</code></pre>
                        <p class="success-msg">✓ Validation passed, event ACKed</p>
                    </div>
                </div>
            </div>

            <div class="schema-insight">
                <p><strong>Key Insight:</strong> KWIKE_PROMPT and KWIKE_SCHEMA_SPEC work together as a complete contract. The prompt tells the LLM WHAT to do, the schema tells it HOW to respond. Validation errors point to documentation, creating a tight feedback loop that enables reliable agent behavior without manual intervention.</p>
            </div>
        </div>

        <div class="interactive-demo">
            <h3>Interactive Demo: LLM Learning Process</h3>
            <p>Watch how an LLM learns kwike and scaffolds a new consumer:</p>

            <div class="demo-steps">
                <div class="demo-step" data-step="1">
                    <div class="step-header">
                        <span class="step-badge">Step 1</span>
                        <h4>LLM Reads Quick Help</h4>
                    </div>
                    <div class="step-body">
                        <div class="terminal-output">
                            <pre><code>$ kwike --help

kwike - Agent orchestration via event queues

Usage:
  kwike [command]

Core Commands:
  daemon      Start the event daemon
  dispatch    Send events to daemon from stdin
  watch       Poll a command and dispatch output
  consume     Process events with LLM subprocess

Management:
  events      Query and manage events
  sessions    Manage consumer sessions
  config      Validate configuration

Documentation:
  doc         View embedded documentation
  version     Show version info

Run 'kwike [command] --help' for command details
Run 'kwike doc' for usage patterns and examples</code></pre>
                        </div>
                        <p class="step-description">The LLM discovers the four primitives and sees that <code>kwike doc</code> has deeper documentation.</p>
                    </div>
                </div>

                <div class="demo-step" data-step="2">
                    <div class="step-header">
                        <span class="step-badge">Step 2</span>
                        <h4>LLM Reads Embedded Docs</h4>
                    </div>
                    <div class="step-body">
                        <div class="terminal-output">
                            <pre><code>$ kwike doc

Available topics:
  consume     Consumer configuration and lifecycle
  daemon      Event daemon architecture
  dispatch    Sending events to the queue
  patterns    Workflow patterns (linear, review-loop, etc.)
  uniform     Template syntax and available context
  watch       Polling commands for event dispatch

$ kwike doc consume

# Consumer Configuration

Consumers poll the daemon for events, render uniform
templates, and supervise LLM subprocesses.

## Quick Start
  kwike consume generate ./agents implementer --types my.task
  $EDITOR ./agents/implementer/uniform.md
  kwike consume --config ./agents/implementer/consumer.yaml

## Configuration Reference
  name: implementer
  source:
    daemon: local
    types: [my.task]
  uniform:
    path: ./uniform.md
  llm:
    command: claude
    args: ["--print"]
...</code></pre>
                        </div>
                        <p class="step-description">The LLM learns the workflow: generate → edit uniform → run consumer. All documentation is embedded in the binary.</p>
                    </div>
                </div>

                <div class="demo-step" data-step="3">
                    <div class="step-header">
                        <span class="step-badge">Step 3</span>
                        <h4>LLM Scaffolds a Consumer</h4>
                    </div>
                    <div class="step-body">
                        <div class="terminal-output">
                            <pre><code>$ kwike consume generate ./agents implementer --types beads.task

Created ./agents/implementer/
  ├── consumer.yaml    # Consumer configuration
  └── uniform.md       # Prompt template (edit this!)

Next steps:
  1. Edit uniform.md with task-specific instructions
  2. Start daemon: kwike daemon
  3. Run consumer: kwike consume --config ./agents/implementer/consumer.yaml</code></pre>
                        </div>
                        <p class="step-description">The scaffolder creates the directory structure with a config file and uniform template ready for customization.</p>
                    </div>
                </div>

                <div class="demo-step" data-step="4">
                    <div class="step-header">
                        <span class="step-badge">Step 4</span>
                        <h4>LLM Customizes the Uniform</h4>
                    </div>
                    <div class="step-body">
                        <div class="terminal-output">
                            <pre><code># ./agents/implementer/uniform.md

You are an implementing agent for bead {{.Event.id}}.

## Task
**{{.Event.title}}**

{{.Event.description}}

## Instructions
1. Read the relevant code files
2. Implement the requested changes
3. Write tests if applicable
4. Commit with a descriptive message

## Output Contract
Your final line MUST be exactly one of:
- DONE: &lt;one-line summary of what you did&gt;
- BLOCKED: &lt;reason you cannot proceed&gt;</code></pre>
                        </div>
                        <p class="step-description">The uniform defines the agent's role, injects event data via Go templates, and specifies the required output format.</p>
                    </div>
                </div>

                <div class="demo-step" data-step="5">
                    <div class="step-header">
                        <span class="step-badge">Step 5</span>
                        <h4>Consumer Processes Events</h4>
                    </div>
                    <div class="step-body">
                        <div class="terminal-output">
                            <pre><code>$ kwike daemon &
$ kwike consume --config ./agents/implementer/consumer.yaml

[consumer] Polling for events: beads.task
[consumer] Received event 019abc... (beads.task)
[consumer] Rendering uniform with event data
[consumer] Spawning: claude --print
[consumer] LLM output: DONE: Added validation to user input form
[consumer] Event acknowledged, emitting beads.task.done
[consumer] Polling for events: beads.task
...</code></pre>
                        </div>
                        <p class="step-description">The consumer polls for events, renders the uniform with event data, spawns the LLM, and acknowledges on success. Fresh session per task.</p>
                    </div>
                </div>
            </div>

            <div class="philosophy-highlight">
                <h4>The Crash-Only, Contract-First Philosophy</h4>
                <div class="philosophy-points">
                    <div class="philosophy-point">
                        <span class="point-icon">🔄</span>
                        <div class="point-text">
                            <h5>Fresh Sessions Per Task</h5>
                            <p>Each event spawns a new LLM subprocess. No accumulated context, no drift. The LLM sees exactly what it needs for one job.</p>
                        </div>
                    </div>

                    <div class="philosophy-point">
                        <span class="point-icon">📋</span>
                        <div class="point-text">
                            <h5>Contracts, Not Conversations</h5>
                            <p>Uniforms + schemas define the contract. The LLM doesn't guess—it receives explicit instructions and output requirements.</p>
                        </div>
                    </div>

                    <div class="philosophy-point">
                        <span class="point-icon">🔧</span>
                        <div class="point-text">
                            <h5>Deterministic Validation</h5>
                            <p>Robots return structured data. Deterministic code validates. Failures trigger retries. No ambiguity, no manual debugging.</p>
                        </div>
                    </div>

                    <div class="philosophy-point">
                        <span class="point-icon">🤖</span>
                        <div class="point-text">
                            <h5>Robots, Not Companions</h5>
                            <p>Treat LLMs like convenience store workers—get in, get what you need, get out. Higher cost per task, but predictable and repeatable.</p>
                        </div>
                    </div>
                </div>
            </div>

            <div class="demo-conclusion">
                <blockquote>
                    "Kwike treats the LLM as a subprocess that receives clear contracts rather than relying on the LLM to figure things out. This crash-only, contract-first approach reduces context drift and enables reliable agent orchestration at scale."
                </blockquote>
            </div>
        </div>
    `;

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('onboarding-content');
        if (container) {
            container.innerHTML = onboardingContent;
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
