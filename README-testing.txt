Run all the tests like so in the container:

$ ./test/bats/bin/bats test/test.bats

Showing the output:

$ TEST_OVERRIDE_RUST_LOG=trace ./test/bats/bin/bats --verbose-run --show-output-of-passing-tests -r test/test.bats 2>&1 | less
