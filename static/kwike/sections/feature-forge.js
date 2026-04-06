// Feature Forge Section - Failure: Complex pipeline but unreliable JSON contracts
document.addEventListener('DOMContentLoaded', () => {
    const container = document.getElementById('feature-forge-content');
    if (!container) return;

    container.innerHTML = `
        <div class="project-header">
            <span class="status-badge failure">Failure</span>
            <p class="project-tagline">Ambition outpaced enforcement: a 4-stage pipeline that collapsed under its own complexity.</p>
            <a href="https://git.sr.ht/~ninjapanzer/feature-forge" class="repo-link" target="_blank" rel="noopener">View Repository →</a>
        </div>

        <div class="project-overview">
            <div class="what-card">
                <h3>What It Attempted</h3>
                <p>
                    A fully automated feature development pipeline: take a feature request,
                    refine it into specs, implement code, review for issues, verify correctness.
                    Four agents, four stages, zero human intervention.
                </p>
            </div>
        </div>

        <div class="pipeline-diagram">
            <h3>The Pipeline</h3>
            <div class="pipeline-flow">
                <div class="pipeline-stage">
                    <div class="stage-number">1</div>
                    <div class="stage-content">
                        <h4>Refine</h4>
                        <p>Parse feature request → structured spec</p>
                    </div>
                </div>
                <span class="pipeline-arrow">→</span>
                <div class="pipeline-stage">
                    <div class="stage-number">2</div>
                    <div class="stage-content">
                        <h4>Implement</h4>
                        <p>Spec → working code</p>
                    </div>
                </div>
                <span class="pipeline-arrow">→</span>
                <div class="pipeline-stage">
                    <div class="stage-number">3</div>
                    <div class="stage-content">
                        <h4>Review</h4>
                        <p>Code → issues list</p>
                    </div>
                </div>
                <span class="pipeline-arrow">→</span>
                <div class="pipeline-stage">
                    <div class="stage-number">4</div>
                    <div class="stage-content">
                        <h4>Verify</h4>
                        <p>Issues → pass/fail</p>
                    </div>
                </div>
            </div>
        </div>

        <div class="kwike-concepts">
            <h3>Kwike Concepts Attempted</h3>
            <div class="concepts-grid">
                <div class="concept-card">
                    <h4>Multi-Consumer Chains</h4>
                    <p>Each stage consumes the previous stage's output. Events flow through
                    the pipeline, each consumer transforming data for the next.</p>
                </div>
                <div class="concept-card">
                    <h4>Retry Logic</h4>
                    <p>Failed stages could retry with backoff. The review stage might
                    send code back to implement for fixes.</p>
                </div>
                <div class="concept-card">
                    <h4>JSON Contracts</h4>
                    <p>Each stage expected specific JSON schemas. Refine outputs a spec object;
                    Review outputs an issues array. Structured data between agents.</p>
                </div>
            </div>
        </div>

        <div class="lesson-box failure-analysis">
            <h4>Why It Failed</h4>
            <div class="failure-points">
                <div class="failure-point">
                    <span class="failure-icon">❌</span>
                    <div>
                        <strong>Unreliable JSON Adherence</strong>
                        <p>
                            LLMs don't reliably produce valid JSON, especially under complex prompts.
                            The refine stage might return prose instead of structured data.
                            The implement stage might wrap code in markdown instead of raw output.
                        </p>
                    </div>
                </div>
                <div class="failure-point">
                    <span class="failure-icon">❌</span>
                    <div>
                        <strong>No Schema Enforcement</strong>
                        <p>
                            Contracts were specified in prompts, not validated at runtime.
                            When an agent produced invalid output, the next stage failed silently
                            or produced garbage. No feedback loop to correct the upstream agent.
                        </p>
                    </div>
                </div>
                <div class="failure-point">
                    <span class="failure-icon">❌</span>
                    <div>
                        <strong>Cascading Failures</strong>
                        <p>
                            One malformed output corrupted the entire pipeline. By the time
                            verify ran, the accumulated drift made results meaningless.
                            The pipeline was only as strong as its weakest stage.
                        </p>
                    </div>
                </div>
            </div>
        </div>

        <div class="lesson-box key-insight">
            <h4>Lesson Learned</h4>
            <p>
                <strong>Contract validation needs enforcement, not just specification.</strong>
            </p>
            <p>
                Telling an agent "output JSON matching this schema" isn't enough.
                You need runtime validation that rejects bad output and forces retry.
                The agent must feel the constraint, not just read about it.
            </p>
            <div class="comparison-table">
                <table>
                    <thead>
                        <tr>
                            <th>Feature Forge (Failed)</th>
                            <th>What Should Have Been</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>Schema in prompt only</td>
                            <td>Schema validation on output</td>
                        </tr>
                        <tr>
                            <td>Hope-based contracts</td>
                            <td>Enforced contracts with retry</td>
                        </tr>
                        <tr>
                            <td>Silent failures</td>
                            <td>Explicit error events</td>
                        </tr>
                        <tr>
                            <td>Trust the agent</td>
                            <td>Verify then trust</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>

        <div class="project-status">
            <h3>Status</h3>
            <p>
                <strong>Abandoned but instructive.</strong> Feature Forge proved that
                multi-agent pipelines are possible but require infrastructure that
                kwike didn't yet have. The failure directly informed the schema
                validation patterns now documented in kwike's onboarding materials.
            </p>
        </div>
    `;
});
