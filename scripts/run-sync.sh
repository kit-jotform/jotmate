#!/bin/bash

# Base directory for all repositories
GITHUB_BASE="$HOME/Documents/Github"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Fork sync result codes (used for parent process decisions after background jobs finish)
FORK_SYNC_EXIT_UPDATED=0
FORK_SYNC_EXIT_UNCHANGED=10
FORK_SYNC_EXIT_ERROR=1

# Function to trim whitespace from a string
trim() {
    local str="$1"
    # Remove leading whitespace
    str="${str#"${str%%[![:space:]]*}"}"
    # Remove trailing whitespace
    str="${str%"${str##*[![:space:]]}"}"
    echo "$str"
}

# Parse command-line arguments
FILTER_PROJECTS=()
FORCE_SYNC_ALL=false
SKIP_FORK_SYNC=false
SKIP_REBASE=false
SKIP_RDS_SYNC=false
SKIP_GIT_FETCH=false
SKIP_DIRTY_SYNC=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --only)
            if [ -z "$2" ]; then
                echo -e "${RED}Error: --only requires a project name or comma-separated list${NC}"
                exit 1
            fi
            # Split comma-separated values and add to filter array
            IFS=',' read -ra PROJECTS <<< "$2"
            for project in "${PROJECTS[@]}"; do
                # Trim whitespace from each project name
                project=$(trim "$project")
                # Only add non-empty projects
                if [ -n "$project" ]; then
                    FILTER_PROJECTS+=("$project")
                fi
            done
            shift 2
            ;;
        --sync-all)
            FORCE_SYNC_ALL=true
            shift
            ;;
        --skip-fork-sync)
            SKIP_FORK_SYNC=true
            shift
            ;;
        --skip-rebase)
            SKIP_REBASE=true
            shift
            ;;
        --skip-rds-sync)
            SKIP_RDS_SYNC=true
            shift
            ;;
        --skip-fetch)
            SKIP_GIT_FETCH=true
            shift
            ;;
        --skip-dirty-sync)
            SKIP_DIRTY_SYNC=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Usage: $0 [--only project1,project2,...] [--sync-all] [--skip-fork-sync] [--skip-rebase] [--skip-rds-sync] [--skip-fetch] [--skip-dirty-sync]"
            exit 1
            ;;
    esac
done

# Supported project names (execution order matters for display)
PROJECTS=("Jotform3" "vendors" "core" "backend" "frontend")
VALID_PROJECTS=("${PROJECTS[@]}")

# Function to check if a project should be included
should_include_project() {
    local project=$1
    # If no filter specified, include all projects
    if [ ${#FILTER_PROJECTS[@]} -eq 0 ]; then
        return 0
    fi
    # Check if project is in filter list
    for filter in "${FILTER_PROJECTS[@]}"; do
        if [ "$project" = "$filter" ]; then
            return 0
        fi
    done
    return 1
}

# Validate filter projects if any are specified
if [ ${#FILTER_PROJECTS[@]} -gt 0 ]; then
    for filter in "${FILTER_PROJECTS[@]}"; do
        VALID=0
        for valid in "${VALID_PROJECTS[@]}"; do
            if [ "$filter" = "$valid" ]; then
                VALID=1
                break
            fi
        done
        if [ $VALID -eq 0 ]; then
            echo -e "${RED}Error: Invalid project name '$filter'${NC}"
            echo -e "Valid projects are: ${VALID_PROJECTS[*]}"
            exit 1
        fi
    done
fi

# Create temp directory for output files
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

project_slug() {
    echo "$1" | tr '[:upper:]' '[:lower:]'
}

fork_log_path() {
    local PROJECT_NAME=$1
    local SLUG=$(project_slug "$PROJECT_NAME")
    echo "$TEMP_DIR/fork_${SLUG}.log"
}

sync_log_path() {
    local PROJECT_NAME=$1
    local SLUG=$(project_slug "$PROJECT_NAME")
    echo "$TEMP_DIR/${SLUG}.log"
}

is_skipped_log() {
    local LOG_FILE=$1
    [ -f "$LOG_FILE" ] && grep -q "^Skipped:" "$LOG_FILE" 2>/dev/null
}

detect_upstream_default_branch() {
    if git rev-parse --verify upstream/main >/dev/null 2>&1; then
        echo "main"
    elif git rev-parse --verify upstream/master >/dev/null 2>&1; then
        echo "master"
    else
        git symbolic-ref refs/remotes/upstream/HEAD 2>/dev/null | sed 's@^refs/remotes/upstream/@@'
    fi
}

# Returns:
#   0 => run ./sync
#   1 => skip ./sync
#   2 => pre-sync pull check failed
#
# Behavior:
# - --sync-all always runs ./sync.
# - Otherwise, when fork is unchanged and repo is clean, try to pull from origin
#   if local is behind before deciding to skip.
prepare_project_sync() {
    local PROJECT_DIR=$1
    local PROJECT_NAME=$2
    local FORK_EXIT_CODE=$3

    if [ "$FORCE_SYNC_ALL" = true ]; then
        return 0
    fi

    # If user explicitly filtered projects, always run requested sync jobs.
    if [ ${#FILTER_PROJECTS[@]} -gt 0 ]; then
        return 0
    fi

    # Only consider skipping when fork sync explicitly reported "unchanged".
    # Any other result (updated, error, no-upstream skip) continues to run ./sync.
    if [ "$FORK_EXIT_CODE" -ne "$FORK_SYNC_EXIT_UNCHANGED" ]; then
        return 0
    fi

    (
        cd "$PROJECT_DIR" >/dev/null 2>&1 || exit 0

        # Any local modifications (including untracked files) should trigger ./sync,
        # unless --skip-dirty-sync is set.
        if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
            if [ "$SKIP_DIRTY_SYNC" = true ]; then
                exit 1  # skip ./sync
            fi
            exit 0
        fi

        # If local branch has no remote tracking branch, or is ahead of it,
        # there are unpushed commits that need syncing.
        CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
        if [ -z "$CURRENT_BRANCH" ] || [ "$CURRENT_BRANCH" = "HEAD" ]; then
            exit 0
        fi
        if ! git rev-parse --verify "origin/$CURRENT_BRANCH" >/dev/null 2>&1; then
            # Try to fetch branch ref once before deciding tracking is unavailable.
            git fetch origin "$CURRENT_BRANCH" >/dev/null 2>&1
        fi
        if ! git rev-parse --verify "origin/$CURRENT_BRANCH" >/dev/null 2>&1; then
            exit 0
        fi

        BEHIND=$(git rev-list --count "HEAD..origin/$CURRENT_BRANCH" 2>/dev/null)
        if [ "${BEHIND:-0}" -gt 0 ]; then
            echo -e "${BLUE}[${PROJECT_NAME}]${NC} Local branch is behind origin/${CURRENT_BRANCH} by ${BEHIND} commit(s), pulling..."
            if ! git pull --ff-only origin "$CURRENT_BRANCH" >/dev/null 2>&1; then
                echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to pull origin/${CURRENT_BRANCH} with --ff-only"
                exit 2
            fi
            echo -e "${GREEN}[${PROJECT_NAME}]${NC} Pulled latest origin/${CURRENT_BRANCH}, running sync"
            exit 0
        fi

        AHEAD=$(git rev-list --count "origin/$CURRENT_BRANCH..HEAD" 2>/dev/null)
        if [ "${AHEAD:-0}" -gt 0 ]; then
            exit 0
        fi

        # Fork sync unchanged, repo clean, nothing ahead/behind => skip ./sync.
        exit 1
    )
}

# Function to sync fork with upstream
sync_fork() {
    local PROJECT_DIR=$1
    local PROJECT_NAME=$2

    if [ "$SKIP_FORK_SYNC" = true ]; then
        echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Fork sync skipped (--skip-fork-sync)"
        return $FORK_SYNC_EXIT_UNCHANGED
    fi

    echo -e "${BLUE}[${PROJECT_NAME}]${NC} Checking if fork sync is needed..."

    cd "$PROJECT_DIR" || {
        echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to enter directory"
        return 1
    }

    # Check if upstream remote exists
    if ! git remote | grep -q "^upstream$"; then
        echo -e "${YELLOW}[${PROJECT_NAME}]${NC} No upstream remote configured, skipping..."
        return 0
    fi

    if [ "$SKIP_GIT_FETCH" = true ]; then
        echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Git fetch skipped (--skip-fetch)"
    else
        # Fetch upstream to check if sync is needed
        echo -e "${BLUE}[${PROJECT_NAME}]${NC} Fetching upstream..."
        if ! git fetch upstream 2>/dev/null; then
            echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to fetch upstream"
            return 1
        fi
    fi
    
    # Detect the default branch name (try main, master, or get from upstream HEAD)
    local DEFAULT_BRANCH=""
    DEFAULT_BRANCH=$(detect_upstream_default_branch)
    if [ -z "$DEFAULT_BRANCH" ]; then
        echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Could not detect default branch, skipping..."
        return 0
    fi
    
    echo -e "${BLUE}[${PROJECT_NAME}]${NC} Using default branch: ${DEFAULT_BRANCH}"
    
    # Check if upstream is ahead of local
    local CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    local LOCAL_COMMIT=$(git rev-parse "$DEFAULT_BRANCH" 2>/dev/null || echo "")
    local UPSTREAM_COMMIT=$(git rev-parse "upstream/$DEFAULT_BRANCH" 2>/dev/null || echo "")
    
    if [ -z "$UPSTREAM_COMMIT" ]; then
        echo -e "${YELLOW}[${PROJECT_NAME}]${NC} No upstream/${DEFAULT_BRANCH} branch found, skipping..."
        return 0
    fi
    
    if [ "$LOCAL_COMMIT" = "$UPSTREAM_COMMIT" ]; then
        echo -e "${GREEN}[${PROJECT_NAME}]${NC} Already up to date with upstream"
        return $FORK_SYNC_EXIT_UNCHANGED
    fi
    
    echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Fork sync needed, starting process..."
    
    # Check for uncommitted changes
    local STASHED=false
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
        echo -e "${BLUE}[${PROJECT_NAME}]${NC} Stashing local changes..."
        git stash push -m "Auto-stash before fork sync" >/dev/null 2>&1
        STASHED=true
    fi
    
    # Checkout default branch
    echo -e "${BLUE}[${PROJECT_NAME}]${NC} Checking out ${DEFAULT_BRANCH} branch..."
    if ! git checkout "$DEFAULT_BRANCH" >/dev/null 2>&1; then
        echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to checkout ${DEFAULT_BRANCH}"
        [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
        return 1
    fi
    
    # Merge upstream
    echo -e "${BLUE}[${PROJECT_NAME}]${NC} Merging upstream/${DEFAULT_BRANCH}..."
    local MERGE_OUTPUT=$(git merge "upstream/$DEFAULT_BRANCH" --no-edit 2>&1)
    local MERGE_STATUS=$?
    
    if [ $MERGE_STATUS -ne 0 ]; then
        echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to merge upstream/${DEFAULT_BRANCH}"
        [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
        return 1
    fi
    
    # Show summary with colored insertions/deletions
    local SUMMARY=$(echo "$MERGE_OUTPUT" | grep -E "^ [0-9]+ files? changed" | head -1)
    if [ -n "$SUMMARY" ]; then
        # Parse and colorize the summary using bash string replacement
        # Git format is like: " 1 file changed, 3 insertions(+), 2 deletions(-)"
        local COLORED_SUMMARY="$SUMMARY"
        # Color the insertions part (number + "insertion(s)" + "(+)")
        if [[ "$COLORED_SUMMARY" =~ ([0-9]+\ insertion[s]?\(\+\)) ]]; then
            local INS_MATCH="${BASH_REMATCH[1]}"
            COLORED_SUMMARY="${COLORED_SUMMARY/$INS_MATCH/${GREEN}${INS_MATCH}${NC}}"
        fi
        # Color the deletions part (number + "deletion(s)" + "(-)")
        if [[ "$COLORED_SUMMARY" =~ ([0-9]+\ deletion[s]?\(-\)) ]]; then
            local DEL_MATCH="${BASH_REMATCH[1]}"
            COLORED_SUMMARY="${COLORED_SUMMARY/$DEL_MATCH/${RED}${DEL_MATCH}${NC}}"
        fi
        echo -e "${CYAN}[${PROJECT_NAME}]${NC} $COLORED_SUMMARY"
    fi
    
    # Push to origin
    echo -e "${BLUE}[${PROJECT_NAME}]${NC} Pushing to origin/${DEFAULT_BRANCH}..."
    if ! git push origin "$DEFAULT_BRANCH" >/dev/null 2>&1; then
        echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to push to origin/${DEFAULT_BRANCH}"
        [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
        return 1
    fi
    echo -e "${GREEN}[${PROJECT_NAME}]${NC} Pushed successfully"
    
    # If we were on a different branch, rebase it and push
    if [ "$CURRENT_BRANCH" != "$DEFAULT_BRANCH" ]; then
        echo -e "${BLUE}[${PROJECT_NAME}]${NC} Switching back to ${CURRENT_BRANCH} branch..."
        if ! git checkout "$CURRENT_BRANCH" >/dev/null 2>&1; then
            echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to checkout ${CURRENT_BRANCH}"
            [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
            return 1
        fi

        if [ "$SKIP_REBASE" = true ]; then
            echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Rebase skipped (--skip-rebase)"
        else
            echo -e "${BLUE}[${PROJECT_NAME}]${NC} Rebasing ${CURRENT_BRANCH} on ${DEFAULT_BRANCH}..."
            if ! git rebase "$DEFAULT_BRANCH" 2>/dev/null; then
                echo -e "${RED}[${PROJECT_NAME}]${NC} Rebase conflict detected, aborting rebase..."
                git rebase --abort 2>/dev/null
                [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
                return 1
            fi

            echo -e "${BLUE}[${PROJECT_NAME}]${NC} Pushing ${CURRENT_BRANCH} with --force-with-lease..."
            if ! git push --force-with-lease origin "$CURRENT_BRANCH" 2>/dev/null; then
                echo -e "${RED}[${PROJECT_NAME}]${NC} Failed to push ${CURRENT_BRANCH}"
                [ "$STASHED" = true ] && git stash pop >/dev/null 2>&1
                return 1
            fi

            echo -e "${GREEN}[${PROJECT_NAME}]${NC} Successfully rebased and pushed ${CURRENT_BRANCH}"
        fi
    fi
    
    # Pop stashed changes if any
    if [ "$STASHED" = true ]; then
        echo -e "${BLUE}[${PROJECT_NAME}]${NC} Restoring uncommitted changes..."
        if ! git stash pop >/dev/null 2>&1; then
            echo -e "${YELLOW}[${PROJECT_NAME}]${NC} Warning: Failed to restore stashed changes"
        fi
    fi
    
    echo -e "${GREEN}[${PROJECT_NAME}]${NC} Fork synced successfully! Now on ${CURRENT_BRANCH} branch"
    return $FORK_SYNC_EXIT_UPDATED
}

# Function to display output for a process
display_output() {
    local NAME=$1
    local LOG_FILE=$2
    local EXIT_CODE=$3
    
    echo -e "\n${CYAN}╔════════════════════════════════════════╗${NC}"
    if is_skipped_log "$LOG_FILE"; then
        echo -e "${CYAN}║${NC} ${YELLOW}○${NC} ${BLUE}${NAME}${NC} - ${YELLOW}SKIPPED${NC}"
    elif [ $EXIT_CODE -eq 0 ]; then
        echo -e "${CYAN}║${NC} ${GREEN}✓${NC} ${BLUE}${NAME}${NC} - ${GREEN}SUCCESS${NC}"
    else
        echo -e "${CYAN}║${NC} ${RED}✗${NC} ${BLUE}${NAME}${NC} - ${RED}FAILED${NC} (exit code: $EXIT_CODE)"
    fi
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
    
    if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
        cat "$LOG_FILE"
    fi
}

display_project_summary_line() {
    local PROJECT_NAME=$1
    local FORK_EXIT_CODE=$2
    local SYNC_LOG_FILE=$3
    local SYNC_EXIT_CODE=$4

    local FORK_FAILED=0
    if [ "$FORK_EXIT_CODE" -ne 0 ] && [ "$FORK_EXIT_CODE" -ne "$FORK_SYNC_EXIT_UNCHANGED" ]; then
        FORK_FAILED=1
    fi

    if [ $FORK_FAILED -eq 1 ] && [ "$SYNC_EXIT_CODE" -ne 0 ]; then
        echo -e "${RED}✗${NC} ${PROJECT_NAME} (fork + sync failed)"
    elif [ $FORK_FAILED -eq 1 ]; then
        echo -e "${RED}✗${NC} ${PROJECT_NAME} (fork failed)"
    elif [ "$SYNC_EXIT_CODE" -ne 0 ]; then
        echo -e "${RED}✗${NC} ${PROJECT_NAME} (sync failed)"
    elif [ "$FORK_EXIT_CODE" -eq "$FORK_SYNC_EXIT_UNCHANGED" ] && is_skipped_log "$SYNC_LOG_FILE"; then
        echo -e "${YELLOW}○${NC} ${PROJECT_NAME} (${YELLOW}SKIPPED${NC})"
    else
        echo -e "${GREEN}✓${NC} ${PROJECT_NAME}"
    fi
}

display_fork_output() {
    local PROJECT_NAME=$1
    local LOG_FILE=$2
    local EXIT_CODE=$3

    if [ "$EXIT_CODE" -eq "$FORK_SYNC_EXIT_UNCHANGED" ]; then
        echo -e "\n${CYAN}╔════════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║${NC} ${YELLOW}○${NC} ${BLUE}${PROJECT_NAME} fork sync${NC} - ${YELLOW}UNCHANGED${NC}"
        echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
        if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
            cat "$LOG_FILE"
        fi
        return
    fi

    display_output "${PROJECT_NAME} fork sync" "$LOG_FILE" "$EXIT_CODE"
}

wait_for_pids_with_progress() {
    local PIDS_ARRAY_NAME=$1
    local WAITING_MESSAGE=$2
    local PROGRESS_LABEL=$3
    local DONE_MESSAGE=$4
    local pid
    local RUNNING

    eval "local -a _pids=(\"\${${PIDS_ARRAY_NAME}[@]}\")"

    if [ ${#_pids[@]} -eq 0 ]; then
        return 0
    fi

    echo -e "\n${YELLOW}${WAITING_MESSAGE}${NC}\n"

    while true; do
        RUNNING=0
        for pid in "${_pids[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                ((RUNNING++))
            fi
        done
        if [ $RUNNING -eq 0 ]; then
            break
        fi
        echo -ne "${CYAN}⏳ ${RUNNING} ${PROGRESS_LABEL}\r${NC}"
        sleep 1
    done

    echo -e "${GREEN}${DONE_MESSAGE}${NC}                    \n"
}

echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║      Syncing Forks with Upstream       ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
if [ ${#FILTER_PROJECTS[@]} -gt 0 ]; then
    echo -e "${YELLOW}Filter: Only syncing: ${FILTER_PROJECTS[*]}${NC}\n"
else
    echo ""
fi
if [ "$FORCE_SYNC_ALL" = true ]; then
    echo -e "${YELLOW}RDS Sync Mode: --sync-all enabled (running ./sync for all repositories)${NC}\n"
fi

# Initialize exit codes (default success for skipped jobs)
FORK_EXIT_CODES=()
SYNC_EXIT_CODES=()
for i in "${!PROJECTS[@]}"; do
    FORK_EXIT_CODES[$i]=0
    SYNC_EXIT_CODES[$i]=0
done

# Initialize arrays to track fork sync processes
FORK_PIDS=()
FORK_PID_INDEXES=()

# Sync forks for included projects in parallel
for i in "${!PROJECTS[@]}"; do
    PROJECT_NAME="${PROJECTS[$i]}"
    if ! should_include_project "$PROJECT_NAME"; then
        continue
    fi

    FORK_LOG_FILE=$(fork_log_path "$PROJECT_NAME")
    sync_fork "$GITHUB_BASE/$PROJECT_NAME" "$PROJECT_NAME" > "$FORK_LOG_FILE" 2>&1 &
    FORK_PID=$!
    FORK_PIDS+=("$FORK_PID")
    FORK_PID_INDEXES+=("$i")
    echo -e "${GREEN}✓${NC} Started fork sync for ${BLUE}${PROJECT_NAME}${NC} (PID: $FORK_PID)"
done

if [ ${#FORK_PIDS[@]} -gt 0 ]; then
    wait_for_pids_with_progress \
        "FORK_PIDS" \
        "⏳ Waiting for fork sync processes to complete..." \
        "fork sync process(es) still running..." \
        "✓ All fork sync processes completed!"

    # Wait for all fork sync processes and capture their exit codes
    for idx in "${!FORK_PIDS[@]}"; do
        pid="${FORK_PIDS[$idx]}"
        wait "$pid"
        FORK_EXIT_CODES[${FORK_PID_INDEXES[$idx]}]=$?
    done
fi

# Display fork sync results
echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║           Fork Sync Results            ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"

for i in "${!PROJECTS[@]}"; do
    PROJECT_NAME="${PROJECTS[$i]}"
    if ! should_include_project "$PROJECT_NAME"; then
        continue
    fi
    FORK_LOG_FILE=$(fork_log_path "$PROJECT_NAME")
    display_fork_output "$PROJECT_NAME" "$FORK_LOG_FILE" "${FORK_EXIT_CODES[$i]}"
done

if [ "$SKIP_RDS_SYNC" = true ]; then
    echo -e "\n${YELLOW}RDS sync skipped (--skip-rds-sync)${NC}\n"
else
    echo -e "\n${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║     Starting Concurrent Sync Jobs      ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}\n"

    # Initialize arrays to track sync processes
    PIDS=()
    SYNC_PID_INDEXES=()

    # Run all sync commands in parallel (each in its own directory)
    # Capture output to separate files (only if included in filter)
    for i in "${!PROJECTS[@]}"; do
        PROJECT_NAME="${PROJECTS[$i]}"
        if [ "$FORCE_SYNC_ALL" != true ] && ! should_include_project "$PROJECT_NAME"; then
            continue
        fi

        SYNC_LOG_FILE=$(sync_log_path "$PROJECT_NAME")
        if prepare_project_sync "$GITHUB_BASE/$PROJECT_NAME" "$PROJECT_NAME" "${FORK_EXIT_CODES[$i]}"; then
            (cd "$GITHUB_BASE/$PROJECT_NAME" && ./sync > "$SYNC_LOG_FILE" 2>&1) &
            PID=$!
            PIDS+=("$PID")
            SYNC_PID_INDEXES+=("$i")
            echo -e "${GREEN}✓${NC} Started ${BLUE}${PROJECT_NAME}/sync${NC} (PID: $PID)"
        else
            PREPARE_STATUS=$?
            if [ "$PREPARE_STATUS" -eq 2 ]; then
                echo "Failed: pull-before-sync check failed (git pull --ff-only origin <current-branch>)" > "$SYNC_LOG_FILE"
                SYNC_EXIT_CODES[$i]=1
                echo -e "${RED}✗${NC} Failed pre-sync pull for ${BLUE}${PROJECT_NAME}${NC}; skipping ./sync"
            else
                echo "Skipped: no upstream updates, no local changes, and nothing to pull from origin" > "$SYNC_LOG_FILE"
                echo -e "${YELLOW}○${NC} Skipped ${BLUE}${PROJECT_NAME}/sync${NC} (no upstream updates, no local changes, nothing to pull)"
            fi
        fi
    done
fi

if [ "$SKIP_RDS_SYNC" != true ]; then
    if [ ${#PIDS[@]} -gt 0 ]; then
        wait_for_pids_with_progress \
            "PIDS" \
            "⏳ Waiting for all processes to complete..." \
            "process(es) still running..." \
            "✓ All processes completed!"

        # Wait for all processes and capture their exit codes
        for idx in "${!PIDS[@]}"; do
            pid="${PIDS[$idx]}"
            wait "$pid"
            SYNC_EXIT_CODES[${SYNC_PID_INDEXES[$idx]}]=$?
        done
    fi

    # Display results with output
    echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║              Sync Results              ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"

    for i in "${!PROJECTS[@]}"; do
        PROJECT_NAME="${PROJECTS[$i]}"
        if [ "$FORCE_SYNC_ALL" != true ] && ! should_include_project "$PROJECT_NAME"; then
            continue
        fi
        display_output "${PROJECT_NAME}/sync" "$(sync_log_path "$PROJECT_NAME")" "${SYNC_EXIT_CODES[$i]}"
    done
fi

# Final summary
echo -e "\n${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║             Final Summary              ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
for i in "${!PROJECTS[@]}"; do
    PROJECT_NAME="${PROJECTS[$i]}"
    if [ "$FORCE_SYNC_ALL" != true ] && ! should_include_project "$PROJECT_NAME"; then
        continue
    fi
    display_project_summary_line "$PROJECT_NAME" "${FORK_EXIT_CODES[$i]}" "$(sync_log_path "$PROJECT_NAME")" "${SYNC_EXIT_CODES[$i]}"
done

# Exit with error if any process failed
FAILED=0
for i in "${!PROJECTS[@]}"; do
    PROJECT_NAME="${PROJECTS[$i]}"
    if [ "$FORCE_SYNC_ALL" != true ] && ! should_include_project "$PROJECT_NAME"; then
        continue
    fi
    if { [ "${FORK_EXIT_CODES[$i]}" -ne 0 ] && [ "${FORK_EXIT_CODES[$i]}" -ne "$FORK_SYNC_EXIT_UNCHANGED" ]; } || [ "${SYNC_EXIT_CODES[$i]}" -ne 0 ]; then
        FAILED=1
    fi
done

if [ $FAILED -eq 1 ]; then
    echo -e "\n${RED}❌ Some sync processes failed${NC}\n"
    exit 1
else
    echo -e "\n${GREEN}🎉 All sync processes completed successfully!${NC}\n"
    exit 0
fi
