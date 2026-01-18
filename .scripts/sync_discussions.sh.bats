#!/usr/bin/env bats

# Load the script under test
SCRIPT_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")" && pwd)"
SCRIPT_PATH="$SCRIPT_DIR/sync_discussions.sh"

# Test fixtures
FIXTURES_DIR=""
TEST_CONTENT_DIR=""
TEST_DATA_DIR=""

setup() {
  # Create temporary test directories
  FIXTURES_DIR="$(mktemp -d)"
  TEST_CONTENT_DIR="$FIXTURES_DIR/content"
  TEST_DATA_DIR="$FIXTURES_DIR/data"
  mkdir -p "$TEST_CONTENT_DIR"
  mkdir -p "$TEST_DATA_DIR/discussion_comments"

  # Set environment variables
  export GITHUB_TOKEN="test_token_12345"
  export REPO_OWNER="testowner"
  export REPO_NAME="testrepo"
  export CONTENT_DIR="$TEST_CONTENT_DIR"
  export DATA_DIR="$TEST_DATA_DIR"
  export COMMENTS_DIR="$TEST_DATA_DIR/discussion_comments"
  export DISCUSSIONS_JSON="$TEST_DATA_DIR/discussions.json"
  export GITHUB_API="https://api.github.com/graphql"
  export DISCUSSION_CATEGORY="Blog Posts"

  # Stub curl BEFORE sourcing script
  stub_curl_github_api

  # Source script functions without executing main
  eval "$(sed '/^main "\$@"$/d' "$SCRIPT_PATH")"
}

teardown() {
  # Clean up test directories
  if [[ -n "$FIXTURES_DIR" && -d "$FIXTURES_DIR" ]]; then
    rm -rf "$FIXTURES_DIR"
  fi
}

# Stub curl to return GitHub API responses based on query
stub_curl_github_api() {
  curl() {
    local payload=""

    # Extract the -d payload
    while [[ $# -gt 0 ]]; do
      case "$1" in
        -d)
          payload="$2"
          shift 2
          ;;
        *)
          shift
          ;;
      esac
    done

    # Return different responses based on query type
    if [[ "$payload" == *"GetRepositoryInfo"* ]]; then
      cat <<'EOF'
{
  "data": {
    "repository": {
      "id": "R_kgDOKgpTas",
      "discussionCategories": {
        "nodes": [
          {
            "id": "DIC_kwDOKgpTas4CXYZ",
            "name": "Blog Posts",
            "slug": "blog-posts"
          }
        ]
      }
    }
  }
}
EOF
    elif [[ "$payload" == *"CreateDiscussion"* ]]; then
      cat <<'EOF'
{
  "data": {
    "createDiscussion": {
      "discussion": {
        "id": "D_kwDOKgpTas4AZnHQ",
        "number": 1,
        "url": "https://github.com/testowner/testrepo/discussions/1",
        "createdAt": "2025-01-18T10:00:00Z",
        "updatedAt": "2025-01-18T10:00:00Z"
      }
    }
  }
}
EOF
    elif [[ "$payload" == *"GetDiscussion"* ]]; then
      cat <<'EOF'
{
  "data": {
    "repository": {
      "discussion": {
        "id": "D_kwDOKgpTas4AZnHQ",
        "url": "https://github.com/testowner/testrepo/discussions/1",
        "updatedAt": "2025-01-18T12:00:00Z",
        "comments": {
          "totalCount": 2,
          "nodes": [
            {
              "id": "DC_kwDOKgpTas4AAABa",
              "author": {
                "login": "testuser1",
                "url": "https://github.com/testuser1",
                "avatarUrl": "https://avatars.githubusercontent.com/u/123?v=4"
              },
              "bodyHTML": "<p>Great article!</p>",
              "createdAt": "2025-01-18T11:00:00Z",
              "updatedAt": "2025-01-18T11:00:00Z",
              "replies": {
                "nodes": [
                  {
                    "id": "DC_kwDOKgpTas4AAABb",
                    "author": {
                      "login": "testuser2",
                      "url": "https://github.com/testuser2",
                      "avatarUrl": "https://avatars.githubusercontent.com/u/456?v=4"
                    },
                    "bodyHTML": "<p>Thanks for reading!</p>",
                    "createdAt": "2025-01-18T11:30:00Z",
                    "updatedAt": "2025-01-18T11:30:00Z"
                  }
                ]
              }
            },
            {
              "id": "DC_kwDOKgpTas4AAABc",
              "author": {
                "login": "testuser3",
                "url": "https://github.com/testuser3",
                "avatarUrl": "https://avatars.githubusercontent.com/u/789?v=4"
              },
              "bodyHTML": "<p>Very informative!</p>",
              "createdAt": "2025-01-18T12:00:00Z",
              "updatedAt": "2025-01-18T12:00:00Z",
              "replies": {
                "nodes": []
              }
            }
          ]
        }
      }
    }
  }
}
EOF
    else
      echo '{"data": null}'
    fi
  }
  export -f curl
}

# Create a test page with discussions enabled
create_test_page() {
  local filename="${1:-test-article.md}"
  local enable_discussions="${2:-true}"
  local has_discussion_number="${3:-false}"
  local discussion_number="${4:-}"

  local file="$TEST_CONTENT_DIR/$filename"

  if [[ "$has_discussion_number" == "true" ]]; then
    cat > "$file" <<EOF
+++
title = "Test Article"
date = 2025-01-18
draft = false
discussion_number = $discussion_number
discussion_url = "https://github.com/testowner/testrepo/discussions/$discussion_number"
[extra]
enable_discussions = $enable_discussions
desc = "A test article"
+++

This is test content.
EOF
  else
    cat > "$file" <<EOF
+++
title = "Test Article"
date = 2025-01-18
draft = false
[extra]
enable_discussions = $enable_discussions
desc = "A test article"
+++

This is test content.
EOF
  fi

  echo "$file"
}

# Test: Script can get repository info
@test "get_repo_info fetches repository ID and category ID" {
  run get_repo_info

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

  [ "$status" -eq 0 ]

  # Parse the response with jq
  local repo_id=$(echo "$output" | jq -r '.data.repository.id')
  local category_id=$(echo "$output" | jq -r '.data.repository.discussionCategories.nodes[] | select(.name == "Blog Posts") | .id')

  [ "$repo_id" = "R_kgDOKgpTas" ]
  [ "$category_id" = "DIC_kwDOKgpTas4CXYZ" ]
}

# Test: Script creates discussion for new page
@test "process_markdown_file creates discussion for page without discussion_number" {
  local test_file=$(create_test_page "new-article.md" "true" "false")

  export REPO_ID="R_kgDOKgpTas"
  export CATEGORY_ID="DIC_kwDOKgpTas4CXYZ"

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

  [ "$status" -eq 0 ]
  [[ "$output" == *"Creating discussion"* ]]
  [[ "$output" == *"Created: #1"* ]]

  # Verify frontmatter was updated
  grep -q 'discussion_number = 1' "$test_file"
  grep -q 'discussion_url = "https://github.com/testowner/testrepo/discussions/1"' "$test_file"

  # Verify comment file was created
  [ -f "$TEST_DATA_DIR/discussion_comments/1.json" ]

  # Verify comment file has correct structure
  local comment_count=$(jq '.discussion.number' "$TEST_DATA_DIR/discussion_comments/1.json")
  [ "$comment_count" = "1" ]
}

# Test: Script fetches existing discussion
@test "process_markdown_file fetches existing discussion when discussion_number exists" {
  local test_file=$(create_test_page "existing-article.md" "true" "true" "1")

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]
  [[ "$output" == *"Discussion exists: #1"* ]]
  [[ "$output" == *"Fetching comments for #1"* ]]
  [[ "$output" == *"Saved 2 comments"* ]]

  # Verify comment file was created with comments
  [ -f "$TEST_DATA_DIR/discussion_comments/1.json" ]

  # Verify comments were saved
  local comment_count=$(jq '.comments | length' "$TEST_DATA_DIR/discussion_comments/1.json")
  [ "$comment_count" = "2" ]

  # Verify nested replies exist
  local reply_count=$(jq '.comments[0].replies.nodes | length' "$TEST_DATA_DIR/discussion_comments/1.json")
  [ "$reply_count" = "1" ]
}

# Test: Script skips pages without enable_discussions
@test "process_markdown_file skips page with enable_discussions=false" {
  local test_file=$(create_test_page "disabled-article.md" "false" "false")

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]
  [[ "$output" == *"Skipping (discussions not enabled)"* ]]

  # Verify no discussion_number was added
  ! grep -q 'discussion_number' "$test_file"
}

# Test: Script skips draft pages
@test "process_markdown_file skips draft pages" {
  local test_file="$TEST_CONTENT_DIR/draft-article.md"
  cat > "$test_file" <<EOF
+++
title = "Draft Article"
draft = true
[extra]
enable_discussions = true
+++

Draft content.
EOF

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]
  [[ "$output" == *"Skipping (draft)"* ]]
}

# Test: Full integration - page without discussion gets one created
@test "integration: full workflow creates discussion and generates data files" {
  local test_file=$(create_test_page "integration-test.md" "true" "false")

  # Simulate get_repo_info call
  local repo_info=$(get_repo_info)
  export REPO_ID=$(echo "$repo_info" | jq -r '.data.repository.id')
  export CATEGORY_ID=$(echo "$repo_info" | jq -r '.data.repository.discussionCategories.nodes[] | select(.name == "Blog Posts") | .id')

  declare -A discussions_map

  # Process the file
  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]

  # Verify discussion was created
  [[ "$output" == *"Created: #1"* ]]

  # Verify frontmatter updated
  grep -q 'discussion_number = 1' "$test_file"

  # Verify comment JSON file created
  [ -f "$TEST_DATA_DIR/discussion_comments/1.json" ]

  # Verify JSON structure is valid
  jq -e '.discussion.id' "$TEST_DATA_DIR/discussion_comments/1.json" >/dev/null
  jq -e '.comments' "$TEST_DATA_DIR/discussion_comments/1.json" >/dev/null
}

# Test: Verify comment data structure is correct
@test "fetch_discussion returns properly structured comment data" {
  run fetch_discussion 1

  [ "$status" -eq 0 ]

  # Verify structure with jq
  echo "$output" | jq -e '.data.repository.discussion.id' >/dev/null
  echo "$output" | jq -e '.data.repository.discussion.comments.nodes[0].author.login' >/dev/null
  echo "$output" | jq -e '.data.repository.discussion.comments.nodes[0].replies.nodes[0]' >/dev/null

  # Verify comment count
  local total=$(echo "$output" | jq '.data.repository.discussion.comments.totalCount')
  [ "$total" = "2" ]
}
