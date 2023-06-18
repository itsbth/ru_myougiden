Output misc/demo.gif
Output misc/demo.ascii
Set Width 1000
Set Height 300
Type "export AKASABI_INDEX=$(mktemp -d)"
Enter
Type "cargo run --quiet -- index --path testdata/JMdict_e_test.gz"
Enter
# NB: vhs doesn't wait for the process to exit, so we need to sleep
Sleep 1s
Type "cargo run --quiet -- search akasabi"
Sleep 500ms
Enter
Sleep 5s
Hide
Type "rm -r $AKASABI_INDEX"
Enter