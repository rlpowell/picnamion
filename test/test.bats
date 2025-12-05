setup() {
  bats_require_minimum_version 1.5.0

  load 'test_helper/bats-support/load'
  load 'test_helper/bats-assert/load'

  # get the containing directory of this file
  # use $BATS_TEST_FILENAME instead of ${BASH_SOURCE[0]} or $0,
  # as those will point to the bats executable's location or the preprocessed file respectively
  DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"
  # make executables in src/ visible to PATH
  PATH="$DIR/../src:$PATH"

  run cargo build
}

DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../
# Save original IFS
OIFS="$IFS"
# Set IFS to only newline
IFS=$'\n'

# Run our test for every file
for file in $(find test_data/ -type f \! -name '*.prefix' \! -name '*.time')
do
  bats_test_function --description "file test for $file" -- output_check "$file"
done

# Restore original IFS
IFS="$OIFS"

output_check() {
  file="$1"
  echo "checking file: $file" 1>&2
  # Set the file to its stored modification time
  touch -d @"$(cat "$file.time")" "$file"
  run -0 ./target/debug/picnamion "$file"
  if [[ "$(cat "$file.prefix")" == "NONE" ]]
  then
    refute_output --partial "INFO: Prefix determined:"
  else
    assert_output --partial "INFO: Prefix determined: $(cat "$file.prefix" || echo "$file.prefix not found")"
  fi
}
