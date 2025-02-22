all: build

build:
  cargo build --all-targets --all-features

export HTMXOLOGY_BASE_URL := "http://localhost:3000"
export SYSTEMFD_LISTEN_ADDR := "tcp::3000"

# Replace the exports above with these to listen on all the interfaces and
# automatically choose a base URL.
#export HTMXOLOGY_BASE_URL := ""
#export SYSTEMFD_LISTEN_ADDR := "tcp::0.0.0.0:3000"

example example:
  # Requires that `dev-setup` has been run at least once.

  systemfd --no-pid -s ${SYSTEMFD_LISTEN_ADDR} -- bacon ex -- {{example}}

doc:
  bacon doc

dev-setup:
  # Install the required tools.
  cargo install --locked just bacon systemfd

publish:
  cargo publish -p htmxology-macros
  cargo publish -p htmxology
