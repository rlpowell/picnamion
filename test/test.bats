setup() {
    load 'test_helper/bats-support/load'
    load 'test_helper/bats-assert/load'

    # get the containing directory of this file
    # use $BATS_TEST_FILENAME instead of ${BASH_SOURCE[0]} or $0,
    # as those will point to the bats executable's location or the preprocessed file respectively
    DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"
    # make executables in src/ visible to PATH
    PATH="$DIR/../src:$PATH"
}

# @test "can run our script" {
#     cd $DIR/../
#     for file in $(find test_data/ -type f \! -name '*.output')
#     do
#       run ./target/debug/picnamion $file
#       assert_output --partial "$(cat "$file.output" || echo "$file.output not found")"
#     done
# }

DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"

    cd $DIR/../
    # Save original IFS
    OIFS="$IFS"
    # Set IFS to only newline
    IFS=$'\n'

    for file in $(find test_data/ -type f \! -name '*.prefix')
    do
      bats_test_function --description "file test for $file" -- output_check "$file"
    done

    # Restore original IFS
    IFS="$OIFS"

output_check() {
    file="$1"
    echo "oc file: $file" 1>&2
      run ./target/debug/picnamion "$file"
      if [[ "$(cat "$file.prefix")" == "NONE" ]]
      then
        refute_output --partial "INFO: Prefix determined:"
      else
        assert_output --partial "INFO: Prefix determined: $(cat "$file.prefix" || echo "$file.prefix not found")"
      fi
}

# @test "foo" {
#   :
# }
