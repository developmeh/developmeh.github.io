// Four Primitives Section Content
(function() {
    const primitivesContent = `
        <div class="primitives-overview">
            <p class="section-intro">
                Kwike provides four independent components that compose through events.
                Each does one thing well. Together they form a complete workflow pipeline.
            </p>
        </div>

        <div class="primitives-grid">
            <div class="primitive-card">
                <div class="primitive-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <rect x="30" y="30" width="40" height="40" rx="5" fill="none" stroke="currentColor" stroke-width="3"/>
                        <circle cx="50" cy="50" r="8" fill="currentColor"/>
                        <path d="M 50 42 L 50 35 M 50 58 L 50 65 M 42 50 L 35 50 M 58 50 L 65 50" stroke="currentColor" stroke-width="2"/>
                    </svg>
                </div>
                <h3>daemon</h3>
                <p class="primitive-tagline">The event store and mail carrier</p>
                <div class="primitive-description">
                    <p>Owns the append-only event log (events.jsonl). Assigns UUIDv7 IDs and timestamps. Serves queries to consumers. Forwards events to peer daemons based on namespace routing (SMTP model: store locally, forward to interested parties).</p>
                    <div class="code-example">
                        <code>kwike daemon</code>
                    </div>
                    <ul class="feature-list">
                        <li>Single writer to events.jsonl</li>
                        <li>Query API over Unix socket</li>
                        <li>Mesh routing with mTLS</li>
                        <li>Graceful shutdown (SIGTERM)</li>
                        <li>Config reload (SIGHUP)</li>
                    </ul>
                </div>
            </div>

            <div class="primitive-card">
                <div class="primitive-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <path d="M 30 50 L 45 35 L 45 45 L 60 45 L 60 30 L 70 40 L 60 50 L 60 55 L 45 55 L 45 65 Z" fill="currentColor"/>
                    </svg>
                </div>
                <h3>dispatch</h3>
                <p class="primitive-tagline">Stock the shelves</p>
                <div class="primitive-description">
                    <p>Reads JSON from stdin, sends to daemon with specified type. Pure filter: no state, no polling. Anything that produces JSON can dispatch events—CI pipelines, webhooks, cron jobs, other consumers.</p>
                    <div class="code-example">
                        <code>echo '{"file": "api.md"}' | kwike dispatch --type docs.update</code>
                    </div>
                    <ul class="feature-list">
                        <li>Stateless filter (stdin → daemon)</li>
                        <li>Arbitrary JSON payloads</li>
                        <li>Thread and parent ID support</li>
                        <li>Address targeting</li>
                    </ul>
                </div>
            </div>

            <div class="primitive-card">
                <div class="primitive-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <circle cx="50" cy="50" r="25" fill="none" stroke="currentColor" stroke-width="3"/>
                        <circle cx="50" cy="50" r="15" fill="none" stroke="currentColor" stroke-width="2"/>
                        <circle cx="50" cy="50" r="5" fill="currentColor"/>
                    </svg>
                </div>
                <h3>watch</h3>
                <p class="primitive-tagline">Dispatch on autopilot</p>
                <div class="primitive-description">
                    <p>Polls a command at intervals and dispatches its output. Convenience wrapper around dispatch for recurring tasks. Perfect for monitoring external systems that don't push events.</p>
                    <div class="code-example">
                        <code>kwike watch "bd ready --json" --type beads.ready --interval 30s</code>
                    </div>
                    <ul class="feature-list">
                        <li>Configurable poll intervals</li>
                        <li>Command output → events</li>
                        <li>Error handling & retries</li>
                        <li>Runs in foreground or daemon mode</li>
                    </ul>
                </div>
            </div>

            <div class="primitive-card">
                <div class="primitive-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <rect x="25" y="30" width="50" height="40" rx="3" fill="none" stroke="currentColor" stroke-width="3"/>
                        <path d="M 35 45 L 42 52 L 60 34" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </div>
                <h3>consume</h3>
                <p class="primitive-tagline">The worker</p>
                <div class="primitive-description">
                    <p>Polls daemon for specific event types, renders prompts from templates (uniforms), supervises LLM subprocesses, validates results, and ACKs successful processing. Flow control prevents overload.</p>
                    <div class="code-example">
                        <code>kwike consume --config implementer.yaml</code>
                    </div>
                    <ul class="feature-list">
                        <li>Event type filtering</li>
                        <li>Uniform template rendering</li>
                        <li>Subprocess supervision</li>
                        <li>Result validation (JSON schemas)</li>
                        <li>Flow control & ACK protocol</li>
                        <li>Crash-only: fresh session per task</li>
                    </ul>
                </div>
            </div>
        </div>

        <div class="architecture-diagram">
            <h3>System Architecture</h3>
            <div class="arch-visual">
                <div class="arch-layer">
                    <div class="arch-component producer">
                        <strong>Producers</strong>
                        <small>watch, dispatch, external systems</small>
                    </div>
                </div>
                <div class="arch-arrow">↓</div>
                <div class="arch-layer">
                    <div class="arch-component daemon">
                        <strong>Daemon</strong>
                        <small>events.jsonl + routing</small>
                    </div>
                </div>
                <div class="arch-arrow">↓</div>
                <div class="arch-layer">
                    <div class="arch-component consumer">
                        <strong>Consumers</strong>
                        <small>consume + LLM agents</small>
                    </div>
                </div>
            </div>
            <div class="arch-note">
                <p>Each component runs independently. They compose via events. Crash any piece—the rest keep running.</p>
            </div>
        </div>
    `;

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('primitives-content');
        if (container) {
            container.innerHTML = primitivesContent;
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
