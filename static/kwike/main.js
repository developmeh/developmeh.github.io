// Main application script - Parallax and Navigation
document.addEventListener('DOMContentLoaded', () => {
    console.log('Kwike presentation loaded');

    // ========================================
    // Theme Detection and Toggle
    // ========================================

    const themeToggle = document.querySelector('.theme-toggle-btn');

    // Check for saved preference, otherwise use system preference
    function getPreferredTheme() {
        const saved = localStorage.getItem('theme');
        if (saved) {
            return saved;
        }
        // Check system preference
        return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    }

    function setTheme(theme) {
        if (theme === 'light') {
            document.documentElement.setAttribute('data-theme', 'light');
        } else {
            document.documentElement.removeAttribute('data-theme');
        }
        localStorage.setItem('theme', theme);
    }

    // Apply initial theme
    setTheme(getPreferredTheme());

    // Toggle handler
    if (themeToggle) {
        themeToggle.addEventListener('click', () => {
            const current = document.documentElement.getAttribute('data-theme');
            const newTheme = current === 'light' ? 'dark' : 'light';
            setTheme(newTheme);
        });
    }

    // Listen for system preference changes
    window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', (e) => {
        // Only auto-switch if user hasn't manually set a preference
        if (!localStorage.getItem('theme')) {
            setTheme(e.matches ? 'light' : 'dark');
        }
    });

    // Get all slides and nav links
    const slides = document.querySelectorAll('.slide');
    const navLinks = document.querySelectorAll('.nav-link');

    // ========================================
    // Create Parallax Layers
    // ========================================

    // Wrap each slide's content in a dedicated parallax layer
    // This separates parallax transforms from entrance animations
    slides.forEach(slide => {
        if (slide.getAttribute('data-parallax')) {
            // Create parallax layer container
            const parallaxLayer = document.createElement('div');
            parallaxLayer.className = 'parallax-layer';

            // Move all existing children to the parallax layer
            while (slide.firstChild) {
                parallaxLayer.appendChild(slide.firstChild);
            }

            // Add the parallax layer back to the slide
            slide.appendChild(parallaxLayer);
        }
    });

    // ========================================
    // Scroll Detection & Section Visibility
    // ========================================

    function updateVisibleSections() {
        const scrollPosition = window.scrollY;
        const windowHeight = window.innerHeight;
        const viewportCenter = scrollPosition + windowHeight / 2;

        let activeSlide = null;
        let bestScore = -Infinity;

        slides.forEach((slide, index) => {
            const slideTop = slide.offsetTop;
            const slideBottom = slideTop + slide.offsetHeight;

            // Check if slide is in viewport
            const isVisible = slideTop < (scrollPosition + windowHeight * 0.75) &&
                            slideBottom > (scrollPosition + windowHeight * 0.25);

            // Add/remove visible class
            if (isVisible) {
                slide.classList.add('visible');
                slide.classList.remove('out-of-view');
            } else {
                slide.classList.remove('visible');
                slide.classList.add('out-of-view');
            }

            // Calculate which section should be highlighted in nav
            // Use section top relative to viewport center - closer to top = higher priority
            if (slideTop <= viewportCenter && slideBottom > scrollPosition) {
                // Section spans the viewport center, or is above it and still visible
                const score = viewportCenter - slideTop;
                if (slideTop <= viewportCenter && score > bestScore) {
                    bestScore = score;
                    activeSlide = slide;
                }
            }
        });

        // Update nav link highlighting
        if (activeSlide) {
            navLinks.forEach(link => link.classList.remove('active'));
            const slideId = activeSlide.getAttribute('id');
            const correspondingLink = document.querySelector(`.nav-link[href="#${slideId}"]`);
            if (correspondingLink) {
                correspondingLink.classList.add('active');
            }
        }
    }

    // ========================================
    // Parallax Scroll Effect
    // ========================================

    function applyParallaxEffect() {
        const scrollPosition = window.scrollY;
        const windowHeight = window.innerHeight;

        slides.forEach(slide => {
            const parallaxSpeed = parseFloat(slide.getAttribute('data-parallax') || '0');

            if (parallaxSpeed > 0) {
                const slideTop = slide.offsetTop;
                const slideHeight = slide.offsetHeight;

                // Calculate section progress (0-1 range) instead of absolute scroll distance
                const sectionProgress = (scrollPosition - slideTop) / slideHeight;
                const normalizedOffset = sectionProgress * parallaxSpeed * windowHeight;

                // Clamp parallax offset to section bounds (30% of section height)
                const maxOffset = slideHeight * 0.3;
                const clampedOffset = Math.max(-maxOffset, Math.min(maxOffset, normalizedOffset));

                // Apply parallax transform to the parallax layer (not the slide)
                // This preserves the slide's entrance animations (translateY 50px->0)
                const parallaxLayer = slide.querySelector('.parallax-layer');
                if (parallaxLayer) {
                    parallaxLayer.style.transform = `translateY(${clampedOffset}px)`;
                }

                // Apply inverse parallax to background layers (::before and ::after)
                // This creates the depth effect
                const beforeOffset = -clampedOffset * 0.5;
                const afterOffset = clampedOffset * 0.3;

                // We can't directly style pseudo-elements, but we can use CSS custom properties
                slide.style.setProperty('--parallax-before', `${beforeOffset}px`);
                slide.style.setProperty('--parallax-after', `${afterOffset}px`);
            }
        });
    }

    // ========================================
    // Smooth Scroll Navigation
    // ========================================

    navLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            const href = link.getAttribute('href');

            // Only handle anchor links (starting with #), let page links work normally
            if (!href.startsWith('#')) {
                return;
            }

            e.preventDefault();

            const targetId = href.substring(1);
            const targetSection = document.getElementById(targetId);

            if (targetSection) {
                targetSection.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
            }
        });
    });

    // ========================================
    // Throttled Scroll Handler
    // ========================================

    let scrollTimeout;
    let lastScrollY = window.scrollY;

    function handleScroll() {
        // Cancel previous timeout
        if (scrollTimeout) {
            window.cancelAnimationFrame(scrollTimeout);
        }

        // Schedule update on next animation frame
        scrollTimeout = window.requestAnimationFrame(() => {
            updateVisibleSections();
            applyParallaxEffect();
            lastScrollY = window.scrollY;
        });
    }

    // ========================================
    // Event Listeners
    // ========================================

    // Initial setup
    updateVisibleSections();
    applyParallaxEffect();

    // Scroll event with throttling
    window.addEventListener('scroll', handleScroll, { passive: true });

    // Resize event (debounced)
    let resizeTimeout;
    window.addEventListener('resize', () => {
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
            updateVisibleSections();
            applyParallaxEffect();
        }, 250);
    });

    // ========================================
    // Keyboard Navigation
    // ========================================

    document.addEventListener('keydown', (e) => {
        const currentSlideIndex = Array.from(slides).findIndex(slide =>
            slide.classList.contains('visible') &&
            slide.getBoundingClientRect().top < window.innerHeight / 2
        );

        if (currentSlideIndex === -1) return;

        let targetSlide = null;

        // Arrow down or Page Down - next slide
        if (e.key === 'ArrowDown' || e.key === 'PageDown') {
            if (currentSlideIndex < slides.length - 1) {
                targetSlide = slides[currentSlideIndex + 1];
            }
        }

        // Arrow up or Page Up - previous slide
        if (e.key === 'ArrowUp' || e.key === 'PageUp') {
            if (currentSlideIndex > 0) {
                targetSlide = slides[currentSlideIndex - 1];
            }
        }

        // Home - first slide
        if (e.key === 'Home') {
            targetSlide = slides[0];
        }

        // End - last slide
        if (e.key === 'End') {
            targetSlide = slides[slides.length - 1];
        }

        if (targetSlide) {
            e.preventDefault();
            targetSlide.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });

    // ========================================
    // Performance Optimization
    // ========================================

    // Reduce motion for users who prefer it
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (prefersReducedMotion) {
        // Disable parallax for users who prefer reduced motion
        slides.forEach(slide => {
            slide.style.transform = 'none';
            slide.removeAttribute('data-parallax');
        });
    }
});
