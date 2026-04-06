// SRHTD Section - Manifestation: Enterprise-ready orchestration
document.addEventListener('DOMContentLoaded', () => {
    const container = document.getElementById('srhtd-content');
    if (!container) return;

    container.innerHTML = `
        <div class="project-header">
            <span class="status-badge success">Production</span>
            <p class="project-tagline">sr.ht todo orchestration starter. Glues kwike to sr.ht tickets via the hut CLI.</p>
            <a href="https://git.sr.ht/~ninjapanzer/srhtd" class="repo-link" target="_blank" rel="noopener">View Repository →</a>
        </div>

        <div class="project-overview">
            <div class="what-card">
                <h3>What It Does</h3>
                <p>
                    Given a spec file or existing ticket, srhtd creates a git worktree, spins up a kwike daemon
                    with five consumer processes, and drives work through a <strong>refine → implement → review → integrate</strong> pipeline.
                </p>
                <p>
                    <strong>Agent-agnostic by design.</strong> srhtd orchestrates <em>processes</em>, not a specific LLM.
                    The pipeline is defined entirely through kwike consumer configs—each consumer's <code>llm.command</code>
                    controls what gets invoked. The event contracts (JSON schemas) are the integration boundary.
                </p>
                <div class="code-example">
                    <code>srhtd start --spec feature.md → refiner → implementer → reviewer → srhtd finish → merged</code>
                </div>
            </div>
        </div>

        <div class="spiritual-successor">
            <h3>Design Philosophy</h3>
            <div class="successor-narrative">
                <p>
                    <strong>Thin CLI bookends.</strong> srhtd handles what shells do well: creating worktrees,
                    managing tickets via <code>hut</code>, dispatching events, and executing merges.
                    All agent cognition lives in kwike consumers.
                </p>
                <p>
                    <strong>Five consumers, one pipeline.</strong> Refiner breaks specs into tasks. Implementers
                    (×3 concurrent) write code. Reviewers (×3 concurrent) approve or reject with session resumption.
                    Integrator resolves conflicts. Notifier tracks state for <code>srhtd status</code>.
                </p>
                <p>
                    <strong>Event contracts as boundaries.</strong> Each consumer has a <code>schema.json</code>
                    defining its output contract. Swap the LLM by changing <code>llm.command</code> in the consumer
                    config—the JSON schema is the integration boundary, not the agent.
                </p>
            </div>
        </div>

        <div class="kwike-concepts">
            <h3>Commands & Recovery</h3>
            <div class="concepts-grid">
                <div class="concept-card">
                    <h4>CLI Commands</h4>
                    <div class="code-example">
                        <code>srhtd start --spec feature.md</code>
                    </div>
                    <p>Creates ticket + worktree, starts daemon + consumers, dispatches <code>srhtd.refine</code>.</p>
                    <div class="code-example">
                        <code>srhtd finish 42</code>
                    </div>
                    <p>Opens diff viewer, triggers integration if conflicts, squash-merges, cleans up.</p>
                </div>
                <div class="concept-card">
                    <h4>Worktree Isolation</h4>
                    <p>
                        Each ticket gets a worktree at <code>../worktree-ticket-&lt;ID&gt;</code> on branch
                        <code>ticket-&lt;ID&gt;/work</code>. Agents commit directly to the work branch.
                        Final merge is a squash into the base branch.
                    </p>
                </div>
                <div class="concept-card">
                    <h4>Model Tiers (Bundled)</h4>
                    <ul class="model-tiers">
                        <li><strong>Opus:</strong> Refiner (spec → tasks), Integrator (conflict resolution)</li>
                        <li><strong>Sonnet ×3:</strong> Implementer (code generation, review-loop resume)</li>
                        <li><strong>Haiku ×3:</strong> Reviewer (approve/reject), Notifier (state tracking)</li>
                    </ul>
                </div>
                <div class="concept-card">
                    <h4>Recovery</h4>
                    <p>
                        <code>srhtd status</code> shows process health + pipeline state.
                        <code>srhtd restart 42</code> kills and restarts consumers.
                        Supervisor tracks PIDs in <code>.srhtd/ticket-&lt;ID&gt;/supervisor.json</code>.
                    </p>
                </div>
            </div>
        </div>

        <div class="dag-uniforms">
            <h3>DAG & Uniforms</h3>

            <div class="dag-section">
                <h4>Process DAG</h4>
                <p class="dag-intro">
                    The full orchestration flow from spec to merged code. <code>srhtd start</code> kicks off the pipeline;
                    kwike consumers handle the agent lifecycle, review loops, and fan-out.
                </p>
                <div class="dag-svg-container">
                    <svg viewBox="0 0 520 840" class="dag-svg">
                        <!-- srhtd start -->
                        <rect x="155" y="10" width="190" height="45" rx="8" fill="#2d3748" stroke="#f1c40f" stroke-width="3"/>
                        <text x="250" y="30" text-anchor="middle" fill="#f1c40f" font-size="13" font-weight="bold">srhtd start</text>
                        <text x="250" y="47" text-anchor="middle" fill="#a0aec0" font-size="10">worktree + ticket</text>

                        <line x1="250" y1="55" x2="250" y2="75" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,80 245,70 255,70" fill="#4a5568"/>

                        <!-- srhtd.refine event -->
                        <rect x="165" y="80" width="170" height="35" rx="8" fill="#2d3748" stroke="#3498db" stroke-width="2"/>
                        <text x="250" y="103" text-anchor="middle" fill="#e2e8f0" font-size="12">srhtd.refine</text>

                        <line x1="250" y1="115" x2="250" y2="135" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,140 245,130 255,130" fill="#4a5568"/>

                        <!-- Opus Refiner -->
                        <rect x="145" y="140" width="210" height="50" rx="8" fill="#2d3748" stroke="#e67e22" stroke-width="3"/>
                        <text x="250" y="163" text-anchor="middle" fill="#e67e22" font-size="15" font-weight="bold">Opus</text>
                        <text x="250" y="180" text-anchor="middle" fill="#a0aec0" font-size="11">refiner: spec → tasks</text>

                        <line x1="250" y1="190" x2="250" y2="210" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,215 245,205 255,205" fill="#4a5568"/>

                        <!-- srhtd.refine.done event -->
                        <rect x="155" y="215" width="190" height="35" rx="8" fill="#2d3748" stroke="#3498db" stroke-width="2"/>
                        <text x="250" y="238" text-anchor="middle" fill="#e2e8f0" font-size="12">srhtd.refine.done</text>

                        <line x1="250" y1="250" x2="250" y2="270" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,275 245,265 255,265" fill="#4a5568"/>

                        <!-- Sonnet Implementer -->
                        <rect x="145" y="275" width="210" height="50" rx="8" fill="#2d3748" stroke="#9b59b6" stroke-width="3"/>
                        <text x="250" y="298" text-anchor="middle" fill="#9b59b6" font-size="15" font-weight="bold">Sonnet ×3</text>
                        <text x="250" y="315" text-anchor="middle" fill="#a0aec0" font-size="11">implementer: sub-branches</text>

                        <line x1="250" y1="325" x2="250" y2="345" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,350 245,340 255,340" fill="#4a5568"/>

                        <!-- srhtd.refine.done.done event -->
                        <rect x="140" y="350" width="220" height="35" rx="8" fill="#2d3748" stroke="#3498db" stroke-width="2"/>
                        <text x="250" y="373" text-anchor="middle" fill="#e2e8f0" font-size="12">srhtd.refine.done.done</text>

                        <line x1="250" y1="385" x2="250" y2="405" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,410 245,400 255,400" fill="#4a5568"/>

                        <!-- Haiku Reviewer -->
                        <rect x="145" y="410" width="210" height="50" rx="8" fill="#2d3748" stroke="#2ecc71" stroke-width="3"/>
                        <text x="250" y="433" text-anchor="middle" fill="#2ecc71" font-size="15" font-weight="bold">Haiku ×3</text>
                        <text x="250" y="450" text-anchor="middle" fill="#a0aec0" font-size="11">reviewer: approve/reject</text>

                        <line x1="250" y1="460" x2="250" y2="480" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,485 245,475 255,475" fill="#4a5568"/>

                        <!-- approved? decision -->
                        <rect x="185" y="485" width="130" height="40" rx="8" fill="#2d3748" stroke="#805ad5" stroke-width="2"/>
                        <text x="250" y="510" text-anchor="middle" fill="#a0aec0" font-size="13">approved?</text>

                        <!-- reject path - review loop -->
                        <line x1="315" y1="505" x2="420" y2="505" stroke="#e74c3c" stroke-width="2"/>
                        <text x="365" y="498" text-anchor="middle" fill="#e74c3c" font-size="11">reject</text>
                        <line x1="420" y1="505" x2="420" y2="295" stroke="#f39c12" stroke-width="2" stroke-dasharray="6,4"/>
                        <line x1="420" y1="295" x2="355" y2="295" stroke="#f39c12" stroke-width="2" stroke-dasharray="6,4"/>
                        <polygon points="355,295 365,290 365,300" fill="#f39c12"/>
                        <text x="435" y="400" text-anchor="start" fill="#f39c12" font-size="10">resume</text>
                        <text x="435" y="415" text-anchor="start" fill="#a0aec0" font-size="9">session</text>

                        <!-- approve path -->
                        <line x1="185" y1="505" x2="80" y2="505" stroke="#2ecc71" stroke-width="2"/>
                        <text x="130" y="498" text-anchor="middle" fill="#2ecc71" font-size="11">approve</text>
                        <line x1="80" y1="505" x2="80" y2="560" stroke="#2ecc71" stroke-width="2"/>
                        <polygon points="80,565 75,555 85,555" fill="#2ecc71"/>

                        <!-- merge sub-branch -->
                        <rect x="20" y="565" width="120" height="35" rx="8" fill="#2d3748" stroke="#4a5568" stroke-width="2"/>
                        <text x="80" y="588" text-anchor="middle" fill="#a0aec0" font-size="11">merge sub-branch</text>

                        <!-- rejoin main flow -->
                        <line x1="80" y1="600" x2="80" y2="620" stroke="#4a5568" stroke-width="2"/>
                        <line x1="80" y1="620" x2="250" y2="620" stroke="#4a5568" stroke-width="2"/>

                        <line x1="250" y1="620" x2="250" y2="640" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,645 245,635 255,635" fill="#4a5568"/>

                        <!-- srhtd finish -->
                        <rect x="155" y="645" width="190" height="45" rx="8" fill="#2d3748" stroke="#f1c40f" stroke-width="3"/>
                        <text x="250" y="665" text-anchor="middle" fill="#f1c40f" font-size="13" font-weight="bold">srhtd finish</text>
                        <text x="250" y="682" text-anchor="middle" fill="#a0aec0" font-size="10">user triggers</text>

                        <line x1="250" y1="690" x2="250" y2="710" stroke="#4a5568" stroke-width="2"/>
                        <polygon points="250,715 245,705 255,705" fill="#4a5568"/>

                        <!-- Opus Integrator -->
                        <rect x="145" y="715" width="210" height="50" rx="8" fill="#2d3748" stroke="#e67e22" stroke-width="3"/>
                        <text x="250" y="738" text-anchor="middle" fill="#e67e22" font-size="15" font-weight="bold">Opus</text>
                        <text x="250" y="755" text-anchor="middle" fill="#a0aec0" font-size="11">integrator: final review</text>

                        <!-- terminal success -->
                        <line x1="145" y1="740" x2="60" y2="740" stroke="#2ecc71" stroke-width="2"/>
                        <line x1="60" y1="740" x2="60" y2="780" stroke="#2ecc71" stroke-width="2"/>
                        <polygon points="60,785 55,775 65,775" fill="#2ecc71"/>

                        <rect x="20" y="785" width="80" height="35" rx="8" fill="#2d3748" stroke="#2ecc71" stroke-width="3"/>
                        <text x="60" y="808" text-anchor="middle" fill="#2ecc71" font-size="12" font-weight="bold">MERGED</text>
                    </svg>
                </div>
                <div class="dag-legend">
                    <div class="legend-item">
                        <span class="legend-symbol" style="background: rgba(241, 196, 15, 0.2); color: #f1c40f;">⌘</span>
                        <span>srhtd CLI: bookends the workflow</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol cycle">↺</span>
                        <span>Review loop: session resumption on reject</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol terminal-success">✓</span>
                        <span>Terminal: squash-merged to master</span>
                    </div>
                </div>
                <a href="assets/srhtd-dag.puml" class="puml-source" download>View PlantUML source</a>
            </div>

            <div class="uniforms-section">
                <h4>Five Consumers</h4>
                <p class="uniforms-intro">
                    Each consumer has a <code>consumer.yaml</code> (kwike config), <code>uniform.md</code> (prompt template),
                    and <code>schema.json</code> (output contract). The uniform constrains the agent's scope.
                </p>
                <div class="uniforms-detail">
                    <div class="uniform-card opus">
                        <div class="uniform-header">
                            <span class="uniform-badge">Refiner</span>
                            <span class="uniform-role">Opus · 5m timeout</span>
                        </div>
                        <div class="uniform-body">
                            <p>Reads spec, decomposes into 3-8 implementable tasks, posts breakdown as ticket comment.</p>
                            <ul>
                                <li><strong>Event:</strong> <code>srhtd.refine</code></li>
                                <li><strong>Output:</strong> Task count, summary, passthrough fields for downstream</li>
                                <li><strong>Constraint:</strong> Tasks must be completable in under 10 minutes each</li>
                            </ul>
                        </div>
                    </div>

                    <div class="uniform-card sonnet">
                        <div class="uniform-header">
                            <span class="uniform-badge">Implementer</span>
                            <span class="uniform-role">Sonnet ×3 · 10m timeout</span>
                        </div>
                        <div class="uniform-body">
                            <p>Reads task breakdown from ticket, implements on work branch. Review-loop pattern: fresh on new tasks, resume on rejection.</p>
                            <ul>
                                <li><strong>Events:</strong> <code>srhtd.refine.done</code>, <code>srhtd.review.rejected</code></li>
                                <li><strong>Output:</strong> Tasks implemented count, summary</li>
                                <li><strong>Constraint:</strong> All work must target the worktree path</li>
                            </ul>
                        </div>
                    </div>

                    <div class="uniform-card haiku">
                        <div class="uniform-header">
                            <span class="uniform-badge">Reviewer</span>
                            <span class="uniform-role">Haiku ×3 · 5m timeout</span>
                        </div>
                        <div class="uniform-body">
                            <p>Reviews full diff on work branch. Approves or rejects—rejection loops back to implementer with session resume.</p>
                            <ul>
                                <li><strong>Events:</strong> <code>srhtd.refine.done.done</code>, <code>srhtd.review.rejected.done</code></li>
                                <li><strong>Output:</strong> Verdict (approved/rejected), reason</li>
                                <li><strong>Constraint:</strong> Be pragmatic—reject only for real issues</li>
                            </ul>
                        </div>
                    </div>

                    <div class="uniform-card opus">
                        <div class="uniform-header">
                            <span class="uniform-badge">Integrator</span>
                            <span class="uniform-role">Opus · 10m timeout</span>
                        </div>
                        <div class="uniform-body">
                            <p>Handles <code>srhtd finish</code> integration—resolves conflicts if any, performs final review before squash-merge.</p>
                            <ul>
                                <li><strong>Event:</strong> <code>srhtd.integrate</code></li>
                                <li><strong>Output:</strong> Merge readiness, conflict resolution summary</li>
                                <li><strong>Constraint:</strong> Preserve semantic intent when resolving conflicts</li>
                            </ul>
                        </div>
                    </div>

                    <div class="uniform-card haiku">
                        <div class="uniform-header">
                            <span class="uniform-badge">Notifier</span>
                            <span class="uniform-role">Haiku · stateless</span>
                        </div>
                        <div class="uniform-body">
                            <p>Tracks pipeline state for <code>srhtd status</code>. Consumes all events, maintains state file for CLI queries.</p>
                            <ul>
                                <li><strong>Events:</strong> <code>srhtd.*</code> (wildcard)</li>
                                <li><strong>Output:</strong> Status updates to state file</li>
                                <li><strong>Constraint:</strong> Lightweight, no blocking operations</li>
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="architecture-diagram">
            <h3>SRHTD Architecture</h3>
            <div class="srhtd-arch">
                <div class="arch-layer">
                    <div class="arch-component cli-layer">
                        <strong>srhtd CLI</strong>
                        <small>Shell orchestrator</small>
                        <ul class="arch-responsibilities">
                            <li>Fetch tickets from sr.ht API</li>
                            <li>Create/destroy worktrees</li>
                            <li>Manage branch lifecycle</li>
                            <li>Execute merges</li>
                            <li>Update ticket status</li>
                        </ul>
                    </div>
                </div>
                <div class="arch-arrow">↕</div>
                <div class="arch-layer">
                    <div class="arch-component kwike-layer">
                        <strong>Kwike Workflows</strong>
                        <small>Agent cognition</small>
                        <ul class="arch-responsibilities">
                            <li>Read ticket requirements</li>
                            <li>Implement features</li>
                            <li>Review code changes</li>
                            <li>Produce structured results</li>
                        </ul>
                    </div>
                </div>
                <div class="arch-arrow">↕</div>
                <div class="arch-layer agents-layer">
                    <div class="arch-component agent-plan">
                        <strong>Opus</strong>
                        <small>Planner</small>
                    </div>
                    <div class="arch-component agent-impl">
                        <strong>Sonnet</strong>
                        <small>Implementer</small>
                    </div>
                    <div class="arch-component agent-review">
                        <strong>Haiku</strong>
                        <small>Reviewer</small>
                    </div>
                </div>
            </div>
        </div>

        <div class="workflow-example">
            <h3>Ticket Lifecycle</h3>
            <div class="lifecycle-flow">
                <div class="lifecycle-step">
                    <span class="step-number">1</span>
                    <div class="step-detail">
                        <h4>Ticket Claimed</h4>
                        <p>srhtd fetches ticket, creates worktree, produces <code>ticket.claimed</code> event</p>
                    </div>
                </div>
                <div class="lifecycle-step">
                    <span class="step-number">2</span>
                    <div class="step-detail">
                        <h4>Implementation</h4>
                        <p>Sonnet agent consumes ticket, writes code in worktree, produces <code>implementation.complete</code></p>
                    </div>
                </div>
                <div class="lifecycle-step">
                    <span class="step-number">3</span>
                    <div class="step-detail">
                        <h4>Review</h4>
                        <p>Haiku agent reviews changes, approves or requests fixes, produces <code>review.complete</code></p>
                    </div>
                </div>
                <div class="lifecycle-step">
                    <span class="step-number">4</span>
                    <div class="step-detail">
                        <h4>Merge & Cleanup</h4>
                        <p>srhtd merges branch, destroys worktree, updates ticket status on sr.ht</p>
                    </div>
                </div>
            </div>
        </div>

        <div class="lesson-box key-insight">
            <h4>The Pattern Crystallized</h4>
            <p>
                <strong>Thin CLI bookends wrapping kwike workflows.</strong>
            </p>
            <p>
                The shell handles what shells do well: filesystem operations, git commands,
                API calls, process management. Kwike handles what agents do well: understanding
                requirements, writing code, evaluating quality. Neither tries to do the other's job.
            </p>
            <div class="pattern-summary">
                <div class="pattern-element">
                    <strong>Before agent:</strong> Shell sets up environment, produces event
                </div>
                <div class="pattern-element">
                    <strong>Agent runs:</strong> Kwike consume with scoped tools, resumed session
                </div>
                <div class="pattern-element">
                    <strong>After agent:</strong> Shell processes result, manages state, produces next event
                </div>
            </div>
        </div>

        <div class="beads-connection">
            <h3>From Skill to Workflow</h3>
            <div class="evolution-narrative">
                <p>
                    <strong>SRHTD was bootstrapped by Claude reading its own documentation.</strong>
                    The beads orchestrator existed as a Claude Code skill—a CLAUDE.md configuration
                    that defined multi-agent coordination. To port it to kwike, Claude read both
                    the skill definition and the kwike agent documentation, then generated the
                    consumer configs and uniform templates.
                </p>
                <ul class="evolution-points">
                    <li><strong>Input:</strong> Beads orchestrator skill + kwike docs</li>
                    <li><strong>Process:</strong> Claude generates consumer.yaml + uniform.md for each agent</li>
                    <li><strong>Output:</strong> Five standalone consumers, portable across environments</li>
                </ul>
                <p>
                    The pattern that once needed Claude Code to exist now runs from cron jobs,
                    CI pipelines, or a developer's terminal. A skill became infrastructure.
                </p>
            </div>
        </div>

        <div class="project-status">
            <h3>Status</h3>
            <p>
                <strong>Production-ready orchestration.</strong> SRHTD represents kwike's
                patterns at their most mature: crash-only design, session resumption,
                tool scoping, event-driven coordination. It's the answer to "what does
                a real kwike deployment look like?"
            </p>
        </div>
    `;
});
