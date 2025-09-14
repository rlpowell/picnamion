$ cargo clean
$ RUSTFLAGS="-C instrument-coverage" cargo build

Do stuff, maybe: ./test/bats/bin/bats test/test.bats

$ rust-profdata merge default_* -o profdata ; rust-cov show -Xdemangler=rustfilt --object target/debug/picnamion -instr-profile=profdata --format=text --sources src/main.rs --use-color | less -R

And then to put it back:

$ rm -f default_* profdata
$ cargo clean
$ cargo build
