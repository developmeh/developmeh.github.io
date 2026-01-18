#!/usr/bin/env bash
set -euo pipefail

# Configuration
GITHUB_API="https://api.github.com/graphql"
DISCUSSION_CATEGORY="Blog Posts"
DATA_DIR="data"
COMMENTS_DIR="$DATA_DIR/discussion_comments"
DISCUSSIONS_JSON="$DATA_DIR/discussions.json"
CONTENT_DIR="content"

# Ensure output directories exist
mkdir -p "$COMMENTS_DIR"

# Colors for logging
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# GraphQL query helper
graphql_query() {
  local query="$1"
  local variables="${2:-{}}"

  curl -s -X POST "$GITHUB_API" \
    -H "Authorization: bearer $GITHUB_TOKEN" \
    -H "Content-Type: application/json" \
    -d "$(jq -n --arg query "$query" --argjson variables "$variables" \
      '{query: $query, variables: $variables}')"
}

# Get repository ID and category ID
get_repo_info() {
  local query='query GetRepositoryInfo($owner: String!, $name: String!) {
    repository(owner: $owner, name: $name) {
      id
      discussionCategories(first: 20) {
        nodes {
          id
          name
          slug
        }
      }
    }
  }'

  local variables=$(jq -n \
    --arg owner "$REPO_OWNER" \
    --arg name "$REPO_NAME" \
    '{owner: $owner, name: $name}')

  graphql_query "$query" "$variables"
}

# Create discussion
create_discussion() {
  local title="$1"
  local body="$2"
  local repo_id="$3"
  local category_id="$4"

  local query='mutation CreateDiscussion($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!) {
    createDiscussion(input: {
      repositoryId: $repositoryId,
      categoryId: $categoryId,
      title: $title,
      body: $body
    }) {
      discussion {
        id
        number
        url
        createdAt
        updatedAt
      }
    }
  }'

  local variables=$(jq -n \
    --arg repositoryId "$repo_id" \
    --arg categoryId "$category_id" \
    --arg title "$title" \
    --arg body "$body" \
    '{repositoryId: $repositoryId, categoryId: $categoryId, title: $title, body: $body}')

  graphql_query "$query" "$variables"
}

# Fetch discussion comments
fetch_discussion() {
  local number="$1"

  local query='query GetDiscussion($owner: String!, $name: String!, $number: Int!) {
    repository(owner: $owner, name: $name) {
      discussion(number: $number) {
        id
        url
        updatedAt
        comments(first: 100) {
          totalCount
          nodes {
            id
            author {
              login
              url
              avatarUrl
            }
            bodyHTML
            createdAt
            updatedAt
            replies(first: 100) {
              nodes {
                id
                author {
                  login
                  url
                  avatarUrl
                }
                bodyHTML
                createdAt
                updatedAt
              }
            }
          }
        }
      }
    }
  }'

  local variables=$(jq -n \
    --arg owner "$REPO_OWNER" \
    --arg name "$REPO_NAME" \
    --argjson number "$number" \
    '{owner: $owner, name: $name, number: $number}')

  graphql_query "$query" "$variables"
}

# Extract frontmatter value using dasel
get_frontmatter_value() {
  local file="$1"
  local key="$2"

  # Extract TOML frontmatter between +++
  awk '/^\+\+\+$/,/^\+\+\+$/' "$file" | \
    grep -v '^\+\+\+$' | \
    dasel -r toml -w plain "$key" 2>/dev/null || echo ""
}

# Process markdown file
process_markdown_file() {
  local file="$1"
  local -n discussions_map=$2

  log_info "Processing: $file"

  # Check if discussions enabled
  local enabled=$(get_frontmatter_value "$file" ".extra.enable_discussions")
  if [[ "$enabled" != "true" ]]; then
    log_info "  Skipping (discussions not enabled)"
    return
  fi

  # Extract metadata
  local title=$(get_frontmatter_value "$file" ".title")
  local draft=$(get_frontmatter_value "$file" ".draft")

  # Skip drafts
  if [[ "$draft" == "true" ]]; then
    log_info "  Skipping (draft)"
    return
  fi

  # Get page path (relative to content/)
  local page_path="${file#$CONTENT_DIR/}"
  page_path="${page_path%.md}"

  # Check if discussion already exists in map
  local existing_number=""
  if [[ -n "${discussions_map[$page_path]:-}" ]]; then
    existing_number=$(echo "${discussions_map[$page_path]}" | jq -r '.number')
  fi

  # Create discussion if needed
  if [[ -z "$existing_number" ]]; then
    log_info "  Creating discussion for: $title"

    local page_url="https://developmeh.com/$page_path"
    local body="Discuss this article: $page_url"
    local result=$(create_discussion "$title" "$body" "$REPO_ID" "$CATEGORY_ID")

    # Check for errors
    if echo "$result" | jq -e '.errors' > /dev/null; then
      log_error "  Failed: $(echo "$result" | jq -r '.errors[0].message')"
      return
    fi

    local discussion_data=$(echo "$result" | jq '.data.createDiscussion.discussion')
    existing_number=$(echo "$discussion_data" | jq -r '.number')

    # Add to map
    discussions_map[$page_path]=$(echo "$discussion_data" | jq '{
      id: .id,
      number: .number,
      url: .url,
      category: "'"$DISCUSSION_CATEGORY"'",
      created_at: .createdAt,
      updated_at: .updatedAt,
      comment_count: 0
    }')

    log_info "  Created: #$existing_number"
  else
    log_info "  Discussion exists: #$existing_number"
  fi

  # Fetch comments
  log_info "  Fetching comments for #$existing_number"
  local discussion_json=$(fetch_discussion "$existing_number")

  # Check for errors
  if echo "$discussion_json" | jq -e '.errors' > /dev/null; then
    log_error "  Failed to fetch: $(echo "$discussion_json" | jq -r '.errors[0].message')"
    return
  fi

  # Extract and save comment data
  local comment_file="$COMMENTS_DIR/${existing_number}.json"
  echo "$discussion_json" | jq '{
    discussion: {
      id: .data.repository.discussion.id,
      number: '"$existing_number"',
      url: .data.repository.discussion.url,
      updated_at: .data.repository.discussion.updatedAt
    },
    comments: .data.repository.discussion.comments.nodes
  }' > "$comment_file"

  # Update discussions map with comment count
  local comment_count=$(echo "$discussion_json" | jq '.data.repository.discussion.comments.totalCount')
  discussions_map[$page_path]=$(echo "${discussions_map[$page_path]}" | \
    jq --argjson count "$comment_count" '.comment_count = $count')

  log_info "  Saved $comment_count comments to $comment_file"
}

# Main execution
main() {
  log_info "Starting GitHub Discussions sync..."

  # Validate environment
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    log_error "GITHUB_TOKEN not set"
    exit 1
  fi

  if [[ -z "${REPO_OWNER:-}" ]] || [[ -z "${REPO_NAME:-}" ]]; then
    log_error "REPO_OWNER and REPO_NAME must be set"
    exit 1
  fi

  # Get repo info
  log_info "Fetching repository information..."
  REPO_INFO=$(get_repo_info)

  if echo "$REPO_INFO" | jq -e '.errors' > /dev/null; then
    log_error "Failed: $(echo "$REPO_INFO" | jq -r '.errors[0].message')"
    exit 1
  fi

  REPO_ID=$(echo "$REPO_INFO" | jq -r '.data.repository.id')
  CATEGORY_ID=$(echo "$REPO_INFO" | jq -r --arg cat "$DISCUSSION_CATEGORY" \
    '.data.repository.discussionCategories.nodes[] | select(.name == $cat) | .id')

  if [[ -z "$CATEGORY_ID" ]]; then
    log_error "Category '$DISCUSSION_CATEGORY' not found"
    echo "$REPO_INFO" | jq -r '.data.repository.discussionCategories.nodes[] | "  - \(.name)"'
    exit 1
  fi

  log_info "Repository ID: $REPO_ID"
  log_info "Category ID: $CATEGORY_ID"

  # Load existing discussions map
  declare -A discussions_map
  if [[ -f "$DISCUSSIONS_JSON" ]]; then
    while IFS="=" read -r key value; do
      discussions_map[$key]="$value"
    done < <(jq -r 'to_entries | .[] | "\(.key)=\(.value | @json)"' "$DISCUSSIONS_JSON")
  fi

  # Process all markdown files
  log_info "Scanning markdown files..."
  while IFS= read -r file; do
    process_markdown_file "$file" discussions_map
  done < <(find "$CONTENT_DIR" -type f -name "*.md" ! -name "_index.md")

  # Save discussions map
  log_info "Saving discussions map to $DISCUSSIONS_JSON"

  # Build proper JSON from map
  {
    echo "{"
    first=true
    for key in "${!discussions_map[@]}"; do
      if [[ "$first" == true ]]; then
        first=false
      else
        echo ","
      fi
      echo "  \"$key\": ${discussions_map[$key]}"
    done
    echo "}"
  } > "$DISCUSSIONS_JSON"

  log_info "Sync complete!"
}

main "$@"
