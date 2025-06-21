# SEO Improvements for Zola Site

This directory contains templates that enhance the SEO capabilities of the site. These templates are designed to work with the zola_easydocs_theme without modifying the theme itself.

## Features Added

### 1. Enhanced Sitemap (sitemap.xml)
- Added priority tags with conditional logic
- Added changefreq tags with conditional logic
- Supports custom priority and changefreq via frontmatter

### 2. Robots.txt
- Standard robots.txt file that allows crawling of the site
- Points to the sitemap

### 3. Open Graph and Twitter Card Metadata (head_meta.html)
- Adds Open Graph metadata for better sharing on social media
- Adds Twitter Card metadata for better Twitter sharing
- Uses page-specific metadata when available, falls back to site-wide metadata

### 4. Structured Data/Schema.org Markup (structured_data.html)
- Adds basic WebPage schema for better search engine understanding
- Supports custom schema types via frontmatter
- Includes author and publisher information

## How to Use

### In Frontmatter

You can add the following to your page's frontmatter to customize SEO:

```toml
[extra]
# Basic SEO
desc = "A description of the page"
keywords = "keyword1, keyword2, keyword3"

# Open Graph / Twitter Card
image = "path/to/image.jpg"  # Relative to static directory

# Sitemap
sitemap_priority = "0.8"  # Value between 0.0 and 1.0
sitemap_changefreq = "monthly"  # Options: always, hourly, daily, weekly, monthly, yearly, never

# Schema.org
schema_type = "BlogPosting"  # Default is WebPage
```

### In config.toml

The following settings have been added to config.toml:

```toml
[extra]
# Default image for Open Graph and Twitter cards if not specified in page frontmatter
default_og_image = "path/to/image.jpg"
# Logo for structured data
logo = "path/to/logo.jpg"
```

## Template Structure

- `page.html` - Extends the theme's page.html template and adds SEO features
- `section.html` - Extends the theme's section.html template and adds SEO features
- `head_meta.html` - Contains Open Graph and Twitter Card metadata
- `structured_data.html` - Contains structured data/schema.org markup
- `sitemap.xml` - Enhanced sitemap template
- `robots.txt` - Standard robots.txt template