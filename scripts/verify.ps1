$ErrorActionPreference = "Stop"

cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test --workspace --release --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo clippy --workspace --all-targets --release --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

git diff --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
