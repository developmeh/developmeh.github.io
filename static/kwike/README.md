# Kwike Architecture Presentation

A parallax-scrolling presentation about the Kwike event-driven agent orchestration system.

## Deployment

This is a static site with no build step required. To deploy:

### Local Testing

Simply open `index.html` in a web browser:

```bash
# From the presentation/ directory
open index.html

# Or use a simple HTTP server (recommended for full feature testing)
python3 -m http.server 8000
# Then visit http://localhost:8000
```

### Static Hosting

Deploy to any static hosting service:

- **Netlify**: Drag and drop the `presentation/` folder
- **GitHub Pages**: Push to a `gh-pages` branch
- **Vercel**: Import and deploy
- **AWS S3**: Upload and enable static website hosting
- **Any web server**: Copy files to document root

### Requirements

- Modern web browser with JavaScript enabled
- No build tools required
- No server-side processing needed
- Works offline once loaded

## Structure

```
presentation/
├── index.html              # Main HTML structure
├── styles.css              # All CSS including parallax effects
├── main.js                 # Scroll handling and navigation
├── sections/
│   ├── philosophy.js       # Philosophy section content
│   ├── events.js           # Event structure content
│   ├── primitives.js       # Four primitives content
│   └── consumers.js        # Consumer patterns content
└── README.md               # This file
```

## Features

- Parallax scrolling effects
- Smooth section transitions
- Keyboard navigation (Arrow keys, Page Up/Down, Home/End)
- Responsive design
- Accessible (respects prefers-reduced-motion)
- Interactive tabs in consumer patterns section

## Browser Compatibility

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers supported

## Customization

All content is in vanilla JavaScript modules under `sections/`. Edit these files to update content without touching the main HTML/CSS structure.
