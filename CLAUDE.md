# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is a Zola-based static site for developmeh.com, a personal blog about software development, devex, software architecture, and technical topics. The site uses the `zola_easydocs_theme` theme with extensive SEO customizations and is deployed to GitHub Pages via Nix-based CI/CD.

## Development Commands

### Local Development
```bash
# Enter development environment (requires Nix with flakes)
nix develop

# Build the site locally
nix develop ./deploy --command zola build --output-dir ./_site

# Serve the site locally with live reload
zola serve

# Check the site for errors
zola check
```

### Testing & Validation
```bash
# Check for typos (typos-lsp is available in the dev shell)
typos

# Validate markdown (marksman LSP is available)
# Use your editor's LSP integration
```

## Architecture

### Site Structure

The repository follows Zola's standard directory structure with custom enhancements:

- **`content/`**: Markdown content organized by topic sections
  - `devex/`: Developer experience articles
  - `i-made-a-thing/`: Project showcases
  - `projects/`: Project documentation
  - `soft-wares/`: Software philosophy and practices
  - `software-architecture/`: Architecture patterns and examples
  - `tech-dives/`: Deep technical explorations
  - `terms-and-afflictions/`: Definitions and concepts
  - Each section has `_index.md` with frontmatter controlling sorting, templates, and behavior

- **`templates/`**: Custom templates that extend the theme
  - SEO-focused templates override theme defaults
  - `page.html` and `section.html`: Main content templates
  - `*_meta.html`: SEO metadata injection (Open Graph, Twitter Cards, Bluesky)
  - `enhanced_structured_data.html`: Schema.org/JSON-LD markup
  - `sitemap.xml`, `robots.txt`: Search engine optimization
  - See `templates/README.md` for detailed SEO customization options

- **`static/`**: Static assets (images, CSS, JS, favicons)
  - `custom.css`: Site-specific styles (currently hides logo on homepage)
  - `custom.js`: Client-side customizations (logo visibility)

- **`themes/zola_easydocs_theme/`**: Theme submodule
  - Do not modify theme files directly
  - Override via templates in root `templates/` directory

### Build System

The site uses Nix flakes for reproducible builds:

- **Root `flake.nix`**: Development environment with `marksman` and `typos-lsp`
  - Imports `./deploy/flake.nix` for deployment shell

- **`deploy/flake.nix`**: Minimal deployment environment with Zola only
  - Used by GitHub Actions for clean builds

- **GitHub Actions** (`.github/workflows/deploy.yml`):
  - Triggered on push to `main`
  - Uses Nix to build site: `nix develop ./deploy --command zola build --output-dir ./_site`
  - Deploys to GitHub Pages automatically

### Configuration

Key configuration lives in `config.toml`:

- **Site metadata**: Base URL, title, author (lines 2-4)
- **Theme**: `zola_easydocs_theme` (line 11)
- **Feeds**: RSS and Atom generation enabled (lines 13-14)
- **Syntax highlighting**: Ruby theme with custom syntaxes from `syntaxes/` (lines 16-21)
- **`[extra]` section**: SEO settings, social profiles, Bluesky config (lines 23-57)
  - Author info, Open Graph defaults, structured data config
  - Social profiles for schema.org `sameAs` property

### SEO Implementation

This site has extensive SEO customizations (documented in `templates/README.md`):

- **Structured data**: WebPage/Article schema, BreadcrumbList, WebSite with SearchAction
- **Social metadata**: Open Graph, Twitter Cards, Bluesky-specific tags
- **Sitemap enhancements**: Priority and changefreq support via frontmatter
- **Canonical URLs**: Prevent duplicate content issues

Page frontmatter can include:
```toml
[extra]
desc = "Page description"
keywords = "keyword1, keyword2"
image = "path/to/image.jpg"
sitemap_priority = "0.8"
sitemap_changefreq = "monthly"
schema_type = "BlogPosting"
bluesky_tags = "tag1, tag2"
```

### Content Organization

Content sections use `_index.md` frontmatter to control behavior:

- **`sort_by`**: How pages in section are ordered (date, weight, title, etc.)
- **`weight`**: Controls section ordering in navigation (lower = higher priority)
- **`template`**: Which template renders the section (usually `section.html`)
- **`insert_anchor_links`**: Adds hover links to headers ("left", "right", "heading", "none")
- **`redirect_to`**: Redirects section root to another page (e.g., root redirects to "home")
- **`generate_feeds`**: Enable RSS/Atom for this section

## Deployment

Deployment is automatic via GitHub Actions:

1. Push to `main` branch
2. GitHub Actions workflow runs
3. Nix builds the site in a clean environment
4. Site deploys to GitHub Pages at https://developmeh.com

The `CNAME` file ensures the custom domain is preserved during deployment.

## Theme Customization

To customize the theme:

1. **Never modify** `themes/zola_easydocs_theme/` directly
2. **Override templates** by creating matching files in root `templates/`
3. **Add custom styles** to `static/custom.css`
4. **Add custom scripts** to `static/custom.js`
5. **Configure via** `config.toml` `[extra]` section

The theme documentation is available at `themes/zola_easydocs_theme/content/` but note this is Zola's documentation, not theme-specific docs.
