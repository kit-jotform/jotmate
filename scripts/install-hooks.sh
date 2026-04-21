#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_PATH="$REPO_ROOT/.git/hooks/pre-commit"

cat > "$HOOK_PATH" << 'HOOK'
#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
step() { echo -e "${YELLOW}▸${NC} $1"; }

step "fmt"
cargo fmt --check 2>&1 || fail "cargo fmt: unformatted files (run 'cargo fmt' to fix)"
pass "fmt"

step "clippy"
cargo clippy --quiet -- -D warnings 2>&1 || fail "cargo clippy: warnings/errors found"
pass "clippy"

step "check"
cargo check --quiet 2>&1 || fail "cargo check: compilation errors found"
pass "check"
HOOK

chmod +x "$HOOK_PATH"
echo "pre-commit hook installed at $HOOK_PATH"
