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

### 4. Additional Meta Tags (additional_meta.html)
- Canonical URL to prevent duplicate content issues
- Language declaration
- Mobile-specific meta tags
- Preconnect links for performance optimization
- Web App Manifest link

### 5. Enhanced Structured Data/Schema.org Markup (enhanced_structured_data.html)
- Comprehensive WebPage/Article schema with additional properties
- BreadcrumbList schema for articles
- WebSite schema with SearchAction
- Organization/LocalBusiness schema with contact information
- Support for social profiles via sameAs property

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
schema_type = "BlogPosting"  # Options: WebPage (default), BlogPosting, Article, Product, etc.
categories = "Category Name"  # For articleSection in BlogPosting/Article schema
```

### In config.toml

The following settings have been added to config.toml:

```toml
[extra]
# Basic SEO
desc = "Site description"
keywords = "keyword1, keyword2, keyword3"

# Open Graph / Twitter Card
default_og_image = "path/to/image.jpg"

# Structured Data
logo = "path/to/logo.jpg"

# Author Information
author_url = "https://example.com/about"
author_image = "path/to/author.jpg"
author_job_title = "Developer"

# Social Profiles for sameAs in structured data
social_profiles = [
  "https://github.com/username",
  "https://twitter.com/username"
]

# Business Information (optional)
business_info = true
business_type = "Organization"  # or LocalBusiness, etc.
business_email = "contact@example.com"
business_telephone = "+1234567890"

[extra.business_address]
street = "123 Main St"
city = "Anytown"
region = "State"
postal_code = "12345"
country = "US"
```

## Template Structure

- `page.html` - Extends the theme's page.html template and adds SEO features
- `section.html` - Extends the theme's section.html template and adds SEO features
- `head_meta.html` - Contains Open Graph and Twitter Card metadata
- `additional_meta.html` - Contains additional meta tags for SEO
- `enhanced_structured_data.html` - Contains enhanced structured data/schema.org markup
- `sitemap.xml` - Enhanced sitemap template
- `robots.txt` - Standard robots.txt template
