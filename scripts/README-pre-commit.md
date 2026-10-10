# Pre-commit Hook

This directory contains the pre-commit hook for the Inklog project.

## Installation

Run the installation script to set up the pre-commit hooks:

```bash
# From the project root directory
./scripts/install-pre-commit.sh
```

The script installs the Python [pre-commit](https://pre-commit.com/) framework (`pip install pre-commit` + `pre-commit install`), which reads `.pre-commit-config.yaml`. The legacy bash hook described in this document (`scripts/pre-commit`) is installed manually by symlink:

```bash
ln -s ../../scripts/pre-commit .git/hooks/pre-commit
```

## What It Checks

Before each commit, the pre-commit hook automatically runs:

1. **Code Formatting** (`cargo fmt`)
   - Ensures all code follows the project's formatting standards
   - Uses `cargo fmt --all -- --check`

2. **Clippy Linting** (`cargo clippy`)
   - Performs static analysis to catch common mistakes
   - Uses strict warning level: `-D warnings`
   - Only checks library code (not test files)

3. **Compilation Check** (`cargo check`)
   - Verifies the code compiles successfully
   - Runs per mutually exclusive database backend group: `cargo check --features 'sqlite,kit,compression,fast-masking'` and `cargo check --features 'postgres,kit,compression,fast-masking'` (`--all-features` is rejected by the database mutual-exclusion guard)

4. **Unit Tests** (`cargo test`)
   - Runs the library unit tests per backend group
   - `cargo test --lib --features 'sqlite,kit,compression,fast-masking' -- --test-threads=4` and the same with `postgres,kit,compression,fast-masking`

## Skipping the Hook

To skip the pre-commit hook for a single commit:

```bash
git commit --no-verify -m "Your commit message"
```

Or use the short form:

```bash
git commit -n -m "Your commit message"
```

## Configuration

The pre-commit hook is located at `scripts/pre-commit`. You can modify it to:

- Add additional checks
- Adjust test timeout
- Change the number of test threads
- Skip specific checks

## Troubleshooting

### Hook not running

Ensure the pre-commit hook is executable:

```bash
chmod +x scripts/pre-commit
```

### Slow pre-commit checks

The hook runs a subset of tests by default. If it's still slow:

1. Reduce the number of test threads: Edit the `--test-threads` value
2. Skip tests temporarily: Comment out the test check

### False positives

If you're seeing false positives from clippy, ensure you're using the latest stable Rust:

```bash
rustup update stable
cargo update
```

## Integration with CI

The pre-commit hook mirrors the checks performed by the GitHub CI pipeline:

- CI Workflow: [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
- Pre-commit checks are a fast subset of CI checks
- Full CI runs on every push to `main` and `develop` branches
