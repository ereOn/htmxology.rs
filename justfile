all: build

build:
  cargo build --all-targets --all-features

example example:
  # Requires that `dev-setup` has been run at least once.
  systemfd --no-pid -s http::3000 -- cargo watch -x 'run --example {{example}} --all-features'

dev-setup:
  # Install the required tools.
  cargo install just cargo-watch systemfd
