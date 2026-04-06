// Intro Section Content - Unix Philosophy Quotes
(function() {
    const quotes = [
        {
            text: "Write programs that do one thing and do it well.",
            author: "Doug McIlroy",
            kwike: "daemon, dispatch, watch, consume — four primitives, each with one job"
        },
        {
            text: "Write programs to work together.",
            author: "Doug McIlroy",
            kwike: "Consumers chain via events. No coupling, just contracts."
        },
        {
            text: "Write programs to handle text streams, because that is a universal interface.",
            author: "Doug McIlroy",
            kwike: "JSON events flow through stdin/stdout. Universal, debuggable, composable."
        },
        {
            text: "To do a new job, build afresh rather than complicate old programs.",
            author: "Doug McIlroy",
            kwike: "Crash-only design. Fresh session per task. No accumulated state."
        },
        {
            text: "Expect the output of every program to become the input to another.",
            author: "Doug McIlroy",
            kwike: "dispatch → daemon → consume → dispatch. Events are the universal interface."
        },
        {
            text: "Don't hesitate to throw away the clumsy parts and rebuild them.",
            author: "Doug McIlroy",
            kwike: "Robots, not companions. Get in, do the job, get out. Rebuild beats repair."
        }
    ];

    const renderQuote = (q, i, isClone = false) => `
        <div class="quote-card${isClone ? ' clone' : ''}" data-index="${i}">
            <div class="quote-card-inner">
                <blockquote>"${q.text}"</blockquote>
                <cite>— ${q.author}</cite>
                <div class="kwike-connection">
                    <span class="connection-label">kwike:</span>
                    <span class="connection-text">${q.kwike}</span>
                </div>
            </div>
        </div>
    `;

    // Clone first and last quotes for infinite scroll effect
    const introContent = `
        <div class="unix-quotes-container">
            <div class="quotes-track">
                ${renderQuote(quotes[quotes.length - 1], quotes.length - 1, true)}
                ${quotes.map((q, i) => renderQuote(q, i)).join('')}
                ${renderQuote(quotes[0], 0, true)}
            </div>
            <div class="quotes-nav">
                ${quotes.map((_, i) => `<button class="quote-dot${i === 0 ? ' active' : ''}" data-index="${i}"></button>`).join('')}
            </div>
            <p class="quote-source">
                <a href="https://en.wikipedia.org/wiki/Unix_philosophy" target="_blank" rel="noopener">Unix Philosophy — Wikipedia</a>
            </p>
        </div>
    `;

    let currentQuote = 0;
    let autoScrollInterval;
    let isTransitioning = false;

    const setupQuoteScroller = () => {
        const track = document.querySelector('.quotes-track');
        const dots = document.querySelectorAll('.quote-dot');
        const cards = document.querySelectorAll('.quote-card:not(.clone)');
        const allCards = document.querySelectorAll('.quote-card');

        if (!track || !dots.length) return;

        // Get card width for calculations
        const getCardWidth = () => allCards[0]?.offsetWidth || 300;

        // Start at first real card (index 1, since 0 is the clone of last)
        const initPosition = () => {
            track.scrollLeft = getCardWidth();
        };

        const scrollToIndex = (index, smooth = true) => {
            // Index 0 = clone of last, 1-6 = real cards, 7 = clone of first
            const targetCard = allCards[index];
            if (targetCard) {
                track.scrollTo({
                    left: targetCard.offsetLeft - track.offsetLeft,
                    behavior: smooth ? 'smooth' : 'instant'
                });
            }
        };

        const updateDots = (realIndex) => {
            dots.forEach((dot, i) => {
                dot.classList.toggle('active', i === realIndex);
            });
        };

        const scrollToQuote = (realIndex) => {
            if (isTransitioning) return;
            currentQuote = realIndex;
            // Real cards are at indices 1 through quotes.length in allCards
            scrollToIndex(realIndex + 1);
            updateDots(realIndex);
        };

        // Handle infinite loop jump
        const handleScrollEnd = () => {
            const cardWidth = getCardWidth();
            const scrollLeft = track.scrollLeft;
            const totalRealWidth = cardWidth * quotes.length;

            // If at clone of last (beginning), jump to real last
            if (scrollLeft <= cardWidth * 0.5) {
                isTransitioning = true;
                track.style.scrollBehavior = 'auto';
                track.scrollLeft = cardWidth * quotes.length;
                track.style.scrollBehavior = 'smooth';
                currentQuote = quotes.length - 1;
                updateDots(currentQuote);
                setTimeout(() => { isTransitioning = false; }, 50);
            }
            // If at clone of first (end), jump to real first
            else if (scrollLeft >= cardWidth * (quotes.length + 0.5)) {
                isTransitioning = true;
                track.style.scrollBehavior = 'auto';
                track.scrollLeft = cardWidth;
                track.style.scrollBehavior = 'smooth';
                currentQuote = 0;
                updateDots(currentQuote);
                setTimeout(() => { isTransitioning = false; }, 50);
            }
        };

        // Dot click handlers
        dots.forEach((dot, i) => {
            dot.addEventListener('click', () => {
                scrollToQuote(i);
                resetAutoScroll();
            });
        });

        // Auto-scroll forward continuously
        const startAutoScroll = () => {
            autoScrollInterval = setInterval(() => {
                if (isTransitioning) return;
                // Always scroll forward, the scroll end handler manages the loop
                const nextIndex = currentQuote + 1;
                if (nextIndex >= quotes.length) {
                    // Scroll to the clone of first, then handler will jump back
                    scrollToIndex(quotes.length + 1);
                    currentQuote = 0;
                    updateDots(0);
                } else {
                    currentQuote = nextIndex;
                    scrollToIndex(nextIndex + 1);
                    updateDots(nextIndex);
                }
            }, 5000);
        };

        const resetAutoScroll = () => {
            clearInterval(autoScrollInterval);
            startAutoScroll();
        };

        // Pause on hover
        track.addEventListener('mouseenter', () => clearInterval(autoScrollInterval));
        track.addEventListener('mouseleave', startAutoScroll);

        // Detect scroll end for infinite loop
        let scrollTimeout;
        track.addEventListener('scroll', () => {
            clearTimeout(scrollTimeout);
            scrollTimeout = setTimeout(handleScrollEnd, 100);

            // Sync dots based on scroll position
            if (!isTransitioning) {
                const cardWidth = getCardWidth();
                const scrollLeft = track.scrollLeft;
                // Account for the clone at the start
                const realIndex = Math.round(scrollLeft / cardWidth) - 1;
                const clampedIndex = Math.max(0, Math.min(quotes.length - 1, realIndex));
                if (clampedIndex !== currentQuote) {
                    currentQuote = clampedIndex;
                    updateDots(clampedIndex);
                }
            }
        });

        // Initialize position and start
        initPosition();
        startAutoScroll();
    };

    // Inject content when DOM is ready
    const injectContent = () => {
        const introSection = document.getElementById('intro');
        if (introSection) {
            const slideContent = introSection.querySelector('.slide-content');
            if (slideContent) {
                // Insert after the about section
                const introAbout = slideContent.querySelector('.intro-about');
                if (introAbout) {
                    introAbout.insertAdjacentHTML('afterend', introContent);
                } else {
                    slideContent.insertAdjacentHTML('beforeend', introContent);
                }
                setupQuoteScroller();
            }
        }
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectContent);
    } else {
        injectContent();
    }
})();
