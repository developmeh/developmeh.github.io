// Message/Event Structure Section Content
(function() {
    const eventsContent = `
        <div class="events-overview">
            <p class="section-intro">
                Kwike uses a structured event format based on WireEvent, combining UUIDv7 identifiers,
                thread tracking, and flexible JSON payloads to create composable event-driven workflows.
            </p>
        </div>

        <div class="events-grid">
            <div class="event-card">
                <h3>WireEvent Format</h3>
                <div class="code-block">
<pre><code>{
  "id": "01HN...",           // UUIDv7 (time-ordered)
  "thread_id": "01HN...",    // Thread grouping
  "parent_id": "01HN...",    // Reply chain
  "type": "beads.task",      // Event type
  "addresses": ["impl"],     // Target consumers
  "payload": {...},          // Arbitrary JSON
  "timestamp": "2025-...",   // ISO8601
  "namespaces": ["beads.*"], // Routing config
  "hops": 0                  // Forwarding count
}</code></pre>
                </div>
            </div>

            <div class="event-card">
                <h3>UUIDv7: Time-Ordered IDs</h3>
                <p>UUIDv7 embeds timestamps, allowing:</p>
                <ul class="feature-list">
                    <li><strong>Natural sorting</strong> - Events sort chronologically by ID</li>
                    <li><strong>No clock sync</strong> - Monotonic within a single daemon</li>
                    <li><strong>Efficient indexes</strong> - Time-range queries via ID prefix</li>
                    <li><strong>Thread causality</strong> - Parent-child relationships preserved</li>
                </ul>
                <div class="visual-timeline">
                    <div class="timeline-bar">
                        <div class="timeline-event" style="left: 10%;">
                            <span class="event-dot"></span>
                            <span class="event-label">01HN...</span>
                        </div>
                        <div class="timeline-event" style="left: 40%;">
                            <span class="event-dot"></span>
                            <span class="event-label">01HO...</span>
                        </div>
                        <div class="timeline-event" style="left: 75%;">
                            <span class="event-dot"></span>
                            <span class="event-label">01HP...</span>
                        </div>
                    </div>
                </div>
            </div>

            <div class="event-card">
                <h3>Event Types</h3>
                <p>Namespaced event types enable selective routing:</p>
                <div class="event-type-examples">
                    <div class="type-example">
                        <code>beads.task</code>
                        <span>→ Route to implementer agents</span>
                    </div>
                    <div class="type-example">
                        <code>docs.update</code>
                        <span>→ Trigger documentation agents</span>
                    </div>
                    <div class="type-example">
                        <code>ci.failed</code>
                        <span>→ Alert fix-it consumers</span>
                    </div>
                    <div class="type-example">
                        <code>custom.*</code>
                        <span>→ Your workflows here</span>
                    </div>
                </div>
            </div>

            <div class="event-card">
                <h3>Result Contracts</h3>
                <p>Consumers return structured results for validation:</p>
                <div class="code-block">
<pre><code>{
  "version": 1,
  "job_id": "...",
  "status": "done|blocked|error",
  "summary": "What happened",
  "metrics": {
    "duration_ms": 1234,
    "tokens_used": 5678
  }
}</code></pre>
                </div>
                <div class="status-indicators">
                    <span class="status done">done</span>
                    <span class="status blocked">blocked</span>
                    <span class="status error">error</span>
                    <span class="status dead-letter">dead_letter</span>
                </div>
            </div>
        </div>

        <div class="event-flow-diagram">
            <h3>Event Flow</h3>
            <div class="flow-visual">
                <div class="flow-step">
                    <div class="flow-box">Producer</div>
                    <div class="flow-arrow">→</div>
                </div>
                <div class="flow-step">
                    <div class="flow-box">Dispatch</div>
                    <div class="flow-arrow">→</div>
                </div>
                <div class="flow-step">
                    <div class="flow-box">Daemon<br/><small>(assigns ID)</small></div>
                    <div class="flow-arrow">→</div>
                </div>
                <div class="flow-step">
                    <div class="flow-box">Consumer<br/><small>(polls + ACKs)</small></div>
                </div>
            </div>
        </div>
    `;

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('events-content');
        if (container) {
            container.innerHTML = eventsContent;
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
