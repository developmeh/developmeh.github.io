// Fruit Stand Section - POC: Git hooks + auto-documentation
document.addEventListener('DOMContentLoaded', () => {
    const container = document.getElementById('fruit-stand-content');
    if (!container) return;

    container.innerHTML = `
        <div class="project-header">
            <span class="status-badge poc">Proof of Concept</span>
            <p class="project-tagline">The simplest question: can an agent maintain its own documentation?</p>
            <a href="https://git.sr.ht/~ninjapanzer/fruit-stand" class="repo-link" target="_blank" rel="noopener">View Repository →</a>
        </div>

        <div class="project-overview">
            <div class="what-card">
                <h3>What It Does</h3>
                <p>
                    A git hook triggers on commit. Kwike reads the changed files and updates
                    the README to reflect the current state of the codebase. No human intervention,
                    no stale documentation.
                </p>
                <div class="code-example">
                    <code>git commit -m "add banana inventory" → README auto-updated</code>
                </div>
            </div>
        </div>

        <div class="kwike-concepts">
            <h3>Kwike Concepts Demonstrated</h3>
            <div class="concepts-grid">
                <div class="concept-card">
                    <h4>Single Consumer</h4>
                    <p>One agent, one job. The documentation consumer watches for code changes
                    and responds with README updates. No coordination overhead.</p>
                </div>
                <div class="concept-card">
                    <h4>Git Hook Integration</h4>
                    <p>Kwike slots into existing workflows. The git hook produces events;
                    kwike handles the rest. Your tools stay your tools.</p>
                </div>
                <div class="concept-card">
                    <h4>Tool Scoping</h4>
                    <p>The agent can read any file but can only write to README.md.
                    Constraints prevent accidents; the agent can't modify source code.</p>
                </div>
            </div>
        </div>

        <div class="architecture-diagram">
            <h3>Architecture Flow</h3>
            <div class="flow-visual">
                <div class="flow-step">
                    <div class="flow-box">
                        <strong>git commit</strong>
                        <small>developer action</small>
                    </div>
                </div>
                <span class="flow-arrow">→</span>
                <div class="flow-step">
                    <div class="flow-box">
                        <strong>post-commit hook</strong>
                        <small>kwike produce</small>
                    </div>
                </div>
                <span class="flow-arrow">→</span>
                <div class="flow-step">
                    <div class="flow-box">
                        <strong>doc-agent</strong>
                        <small>kwike consume</small>
                    </div>
                </div>
                <span class="flow-arrow">→</span>
                <div class="flow-step">
                    <div class="flow-box">
                        <strong>README.md</strong>
                        <small>updated</small>
                    </div>
                </div>
            </div>
        </div>

        <div class="lesson-box key-insight">
            <h4>Key Insight: The Wrapper Pattern</h4>
            <p>
                <strong>Agents shouldn't commit directly.</strong> The original design had the agent
                run <code>git commit</code> after updating the README. This created a loop:
                commit → hook → agent → commit → hook...
            </p>
            <p>
                The fix: a wrapper script handles the commit. The agent writes files;
                the shell orchestrates git. Each layer does one thing.
            </p>
            <div class="code-example">
                <pre><code># Wrong: agent commits
claude --dangerously-skip-permissions -p "update docs then commit"

# Right: wrapper commits
kwike consume doc-agent  # agent writes README
git add README.md && git commit --no-verify -m "docs: auto-update"</code></pre>
            </div>
        </div>

        <div class="project-status">
            <h3>Status</h3>
            <p>
                <strong>Working proof of concept.</strong> Demonstrates that event-driven
                agent workflows integrate cleanly with existing developer tooling.
                The pattern—hook produces, agent consumes, wrapper commits—became
                the foundation for more complex projects.
            </p>
        </div>
    `;
});
