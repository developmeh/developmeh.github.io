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
  export USE_ORG_DISCUSSIONS="false"

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
  if [[ "$payload" == *"FAIL_ME"* ]]; then
    return 1
  elif [[ "$payload" == *"EMPTY_ME"* ]]; then
    echo ""
    return 0
  elif [[ "$payload" == *"GetRepositoryInfo"* ]]; then
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
    elif [[ "$payload" == *"SearchDiscussions"* ]]; then
      # Extract the search query from the variables.query field
      local search_query=$(echo "$payload" | jq -r '.variables.query')

      # Check if searching by URL (contains "developmeh.com")
      if [[ "$search_query" == *"developmeh.com/test-article"* ]]; then
        # Return existing discussion for test-article URL
        cat <<'EOF'
{
  "data": {
    "search": {
      "nodes": [
        {
          "id": "D_kwDOKgpTas4AZnHQ",
          "number": 42,
          "url": "https://github.com/testowner/testrepo/discussions/42",
          "title": "Test Article",
          "body": "Discuss this article: https://developmeh.com/test-article",
          "createdAt": "2025-01-17T10:00:00Z",
          "updatedAt": "2025-01-18T10:00:00Z"
        }
      ]
    }
  }
}
EOF
      elif [[ "$search_query" == *"developmeh.com/multiple-match-article"* ]]; then
        # Return multiple discussions for testing warning
        cat <<'EOF'
{
  "data": {
    "search": {
      "nodes": [
        {
          "id": "D_kwDOKgpTas4AZnHQ",
          "number": 50,
          "url": "https://github.com/testowner/testrepo/discussions/50",
          "title": "Multiple Match Article",
          "body": "Discuss this article: https://developmeh.com/multiple-match-article",
          "createdAt": "2025-01-17T10:00:00Z",
          "updatedAt": "2025-01-18T10:00:00Z"
        },
        {
          "id": "D_kwDOKgpTas4AZnHR",
          "number": 51,
          "url": "https://github.com/testowner/testrepo/discussions/51",
          "title": "Multiple Match Article - Duplicate",
          "body": "Discuss this article: https://developmeh.com/multiple-match-article",
          "createdAt": "2025-01-17T11:00:00Z",
          "updatedAt": "2025-01-18T11:00:00Z"
        }
      ]
    }
  }
}
EOF
      elif [[ "$search_query" == 'repo:testowner/testrepo in:title "Test Article"' ]]; then
        # Return existing discussion for "Test Article" title search (fallback)
        cat <<'EOF'
{
  "data": {
    "search": {
      "nodes": [
        {
          "id": "D_kwDOKgpTas4AZnHQ",
          "number": 42,
          "url": "https://github.com/testowner/testrepo/discussions/42",
          "title": "Test Article",
          "body": "Discuss this article: https://developmeh.com/test-article",
          "createdAt": "2025-01-17T10:00:00Z",
          "updatedAt": "2025-01-18T10:00:00Z"
        }
      ]
    }
  }
}
EOF
      else
        # No results for other searches
        cat <<'EOF'
{
  "data": {
    "search": {
      "nodes": []
    }
  }
}
EOF
      fi
    elif [[ "$payload" == *"GetDiscussion"* ]]; then
      # Extract discussion number from the payload
      local discussion_num=$(echo "$payload" | jq -r '.variables.number')

      # Return different data based on discussion number
      if [[ "$discussion_num" == "42" ]] || [[ "$discussion_num" == "50" ]]; then
        cat <<EOF
{
  "data": {
    "repository": {
      "discussion": {
        "id": "D_kwDOKgpTas4AZnHQ",
        "url": "https://github.com/testowner/testrepo/discussions/$discussion_num",
        "updatedAt": "2025-01-18T12:00:00Z",
        "comments": {
          "totalCount": 2,
          "nodes": [
            {
              "id": "DC_kwDOKgpTas4AAABa",
              "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-123",
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
                    "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-456",
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
              "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-789",
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
        # Default response for other discussion numbers
        cat <<EOF
{
  "data": {
    "repository": {
      "discussion": {
        "id": "D_kwDOKgpTas4AZnHQ",
        "url": "https://github.com/testowner/testrepo/discussions/$discussion_num",
        "updatedAt": "2025-01-18T12:00:00Z",
        "comments": {
          "totalCount": 2,
          "nodes": [
            {
              "id": "DC_kwDOKgpTas4AAABa",
              "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-123",
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
                    "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-456",
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
              "url": "https://github.com/testowner/testrepo/discussions/$discussion_num#discussioncomment-789",
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
      fi
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
  local title="${5:-Test Article}"

  local file="$TEST_CONTENT_DIR/$filename"

  if [[ "$has_discussion_number" == "true" ]]; then
    cat > "$file" <<EOF
+++
title = "$title"
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
title = "$title"
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

# Helper to setup repo info
setup_repo_vars() {
  export REPO_ID="R_kgDOKgpTas"
  export CATEGORY_ID="DIC_kwDOKgpTas4CXYZ"
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
  # Use a different title to avoid matching existing "Test Article" in search
  local test_file=$(create_test_page "new-article.md" "true" "false" "" "Brand New Article")

  setup_repo_vars

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

  [ "$status" -eq 0 ]
  [[ "$output" == *"Searching for existing discussion"* ]]
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
  local test_file=$(create_test_page "existing-article.md" "true" "true" "1" "Existing Article")

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

# Test: Script skips pages with enable_discussions=false
@test "process_markdown_file skips page with enable_discussions=false" {
  local test_file=$(create_test_page "disabled-article.md" "false" "false" "" "Disabled Article")

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]
  [[ "$output" == *"Skipping (discussions explicitly disabled)"* ]]

  # Verify no discussion_number was added
  ! grep -q 'discussion_number' "$test_file"
}

# Test: Script processes pages without enable_discussions (opt-in by default)
@test "process_markdown_file processes page without enable_discussions (opt-in by default)" {
  local test_file="$TEST_CONTENT_DIR/no-enable-field.md"
  cat > "$test_file" <<EOF
+++
title = "No Enable Field"
date = 2025-01-18
draft = false
[extra]
desc = "A test article without enable_discussions field"
+++

This is test content.
EOF

  setup_repo_vars
  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  [ "$status" -eq 0 ]
  [[ "$output" == *"Creating discussion for: No Enable Field"* ]]
  grep -q 'discussion_number = 1' "$test_file"
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
  # Use a different title to avoid matching existing "Test Article" in search
  local test_file=$(create_test_page "integration-test.md" "true" "false" "" "Integration Test Article")

  # Simulate get_repo_info call
  local repo_info=$(get_repo_info)
  export REPO_ID=$(echo "$repo_info" | jq -r '.data.repository.id')
  export CATEGORY_ID=$(echo "$repo_info" | jq -r '.data.repository.discussionCategories.nodes[] | select(.name == "Blog Posts") | .id')

  declare -A discussions_map

  # Process the file
  run process_markdown_file "$test_file" discussions_map

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

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

# Test: search_discussion_by_title finds existing discussion
@test "search_discussion_by_title finds existing discussion by title" {
  run search_discussion_by_title "Test Article"

  [ "$status" -eq 0 ]

  # Verify we got a discussion back
  local discussion_number=$(echo "$output" | jq -r '.data.search.nodes[0].number')
  [ "$discussion_number" = "42" ]

  local discussion_title=$(echo "$output" | jq -r '.data.search.nodes[0].title')
  [ "$discussion_title" = "Test Article" ]
}

# Test: search_discussion_by_title returns empty when no discussion exists
@test "search_discussion_by_title returns empty array when no discussion found" {
  run search_discussion_by_title "Nonexistent Article"

  [ "$status" -eq 0 ]

  # Verify empty results
  local node_count=$(echo "$output" | jq '.data.search.nodes | length')
  [ "$node_count" = "0" ]
}

# Test: process_markdown_file finds and links existing discussion when frontmatter missing
@test "process_markdown_file links existing discussion when discussion_number missing from frontmatter" {
  # Create a page without discussion_number in frontmatter
  local test_file=$(create_test_page "test-article.md" "true" "false")

  setup_repo_vars

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

  [ "$status" -eq 0 ]

  # Should have found existing discussion #42 instead of creating a new one
  [[ "$output" == *"Searching for existing discussion by URL"* ]]
  [[ "$output" == *"Found existing discussion #42"* ]]
  [[ "$output" != *"Creating discussion"* ]]

  # Verify frontmatter was updated with found discussion
  grep -q 'discussion_number = 42' "$test_file"
  grep -q 'discussion_url = "https://github.com/testowner/testrepo/discussions/42"' "$test_file"

  # Verify comments JSON file exists and contains URLs
  local comment_file="$COMMENTS_DIR/42.json"
  [ -f "$comment_file" ]
  
  # Check for comment URL
  local first_comment_url=$(jq -r '.comments[0].url' "$comment_file")
  [ "$first_comment_url" = "https://github.com/testowner/testrepo/discussions/42#discussioncomment-123" ]
  
  # Check for reply URL
  local first_reply_url=$(jq -r '.comments[0].replies.nodes[0].url' "$comment_file")
  [ "$first_reply_url" = "https://github.com/testowner/testrepo/discussions/42#discussioncomment-456" ]
}

# Test: search_discussion_by_url finds existing discussion
@test "search_discussion_by_url finds existing discussion by article URL" {
  run search_discussion_by_url "https://developmeh.com/test-article"

  [ "$status" -eq 0 ]

  # Verify we got a discussion back
  local discussion_number=$(echo "$output" | jq -r '.data.search.nodes[0].number')
  [ "$discussion_number" = "42" ]

  local discussion_body=$(echo "$output" | jq -r '.data.search.nodes[0].body')
  [[ "$discussion_body" == *"https://developmeh.com/test-article"* ]]
}

# Test: search_discussion_by_url returns empty when no discussion exists
@test "search_discussion_by_url returns empty array when no discussion found" {
  run search_discussion_by_url "https://developmeh.com/nonexistent-article"

  [ "$status" -eq 0 ]

  # Verify empty results
  local node_count=$(echo "$output" | jq '.data.search.nodes | length')
  [ "$node_count" = "0" ]
}

# Test: process_markdown_file warns on multiple matches
@test "process_markdown_file warns when multiple discussions match the same URL" {
  # Create a page that will match multiple discussions
  local test_file=$(create_test_page "multiple-match-article.md" "true" "false" "" "Multiple Match Article")

  export REPO_ID="R_kgDOKgpTas"
  export CATEGORY_ID="DIC_kwDOKgpTas4CXYZ"

  declare -A discussions_map

  run process_markdown_file "$test_file" discussions_map

  # Debug output
  echo "Status: $status" >&3
  echo "Output: $output" >&3

  [ "$status" -eq 0 ]

  # Should warn about multiple matches
  [[ "$output" == *"Found 2 discussions matching URL"* ]]
  [[ "$output" == *"Discussion numbers: 50 51"* ]]
  [[ "$output" == *"Using the first match"* ]]

  # Should use the first match (discussion #50)
  grep -q 'discussion_number = 50' "$test_file"
}

# Test: graphql_query handles curl failure
@test "graphql_query handles curl failure" {
  run graphql_query "FAIL_ME"
  echo "Output: $output" >&3
  [ "$status" -ne 0 ]
  [[ "$output" == *"GraphQL request failed"* ]]
}

# Test: graphql_query handles empty response
@test "graphql_query handles empty response" {
  run graphql_query "EMPTY_ME"
  echo "Output: $output" >&3
  [ "$status" -ne 0 ]
  [[ "$output" == *"GraphQL request returned empty response"* ]]
}
