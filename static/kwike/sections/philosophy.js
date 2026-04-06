// Philosophy Section Content
(function() {
    const philosophyContent = `
        <div class="philosophy-grid">
            <div class="philosophy-card" data-principle="crash-only">
                <div class="card-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <circle cx="50" cy="50" r="40" fill="none" stroke="currentColor" stroke-width="3"/>
                        <path d="M 50 30 L 50 60 M 50 70 L 50 75" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>
                    </svg>
                </div>
                <h3>Crash-Only Design</h3>
                <p>Fresh sessions per task. No accumulated context. No drift. Each agent sees exactly what it needs for one job.</p>
                <div class="principle-detail">
                    <code>New instance → Do work → Exit</code>
                </div>
            </div>

            <div class="philosophy-card" data-principle="robots">
                <div class="card-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <rect x="30" y="30" width="40" height="40" rx="5" fill="none" stroke="currentColor" stroke-width="3"/>
                        <circle cx="42" cy="45" r="3" fill="currentColor"/>
                        <circle cx="58" cy="45" r="3" fill="currentColor"/>
                        <path d="M 40 58 Q 50 63 60 58" fill="none" stroke="currentColor" stroke-width="2"/>
                    </svg>
                </div>
                <h3>Robots, Not Companions</h3>
                <p>Treat LLMs like convenience store workers. Get in, get what you need, get out. No ambiance required.</p>
                <div class="principle-detail">
                    Higher cost per task, but predictable and repeatable
                </div>
            </div>

            <div class="philosophy-card" data-principle="composability">
                <div class="card-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <rect x="20" y="35" width="25" height="25" fill="none" stroke="currentColor" stroke-width="2"/>
                        <rect x="55" y="35" width="25" height="25" fill="none" stroke="currentColor" stroke-width="2"/>
                        <path d="M 45 47.5 L 55 47.5" stroke="currentColor" stroke-width="2" marker-end="url(#arrow)"/>
                        <defs>
                            <marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
                                <polygon points="0 0, 10 3, 0 6" fill="currentColor" />
                            </marker>
                        </defs>
                    </svg>
                </div>
                <h3>Unix Primitives</h3>
                <p>Small programs that do one thing well. Compose via pipes and events. Make data complicated, not programs.</p>
                <div class="principle-detail">
                    daemon | dispatch | watch | consume
                </div>
            </div>

            <div class="philosophy-card" data-principle="contracts">
                <div class="card-icon">
                    <svg viewBox="0 0 100 100" class="icon-svg">
                        <rect x="30" y="25" width="40" height="50" fill="none" stroke="currentColor" stroke-width="2"/>
                        <path d="M 38 38 L 62 38 M 38 48 L 62 48 M 38 58 L 52 58" stroke="currentColor" stroke-width="2"/>
                        <path d="M 45 65 Q 50 68 55 65" stroke="currentColor" stroke-width="2" fill="none"/>
                    </svg>
                </div>
                <h3>Schemas Enforce Contracts</h3>
                <p>Robots return structured output. Deterministic code validates. Failures trigger retries. Works after one attempt.</p>
                <div class="principle-detail">
                    JSON schemas + validation = predictability
                </div>
            </div>
        </div>

        <div class="philosophy-quote">
            <blockquote>
                "Drudgery is for robots. Some problems are too hard to code. Chain a robot to that drudgery and enjoy the sunshine."
            </blockquote>
        </div>
    `;

    // Inject content when DOM is ready
    const injectContent = () => {
        const container = document.getElementById('philosophy-content');
        if (container) {
            container.innerHTML = philosophyContent;
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
