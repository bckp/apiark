#!/usr/bin/env fish
set -l repo_root (path resolve (path dirname (status filename))/..)
cd $repo_root; or exit 1
cargo fmt --all --check; or exit 1
cargo test -p apiark-core -p apiark-cli --locked; or exit 1
cargo clippy -p apiark-core -p apiark-cli --all-targets --locked; or exit 1
pnpm --filter @apiark/desktop test; or exit 1
pnpm --filter @apiark/desktop lint; or exit 1
pnpm --filter @apiark/desktop build; or exit 1
if test (uname) = Darwin
    cargo test -p apiark --lib --locked; or exit 1
    cargo check -p apiark --locked; or exit 1
end
