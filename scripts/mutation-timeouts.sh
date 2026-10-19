#!/usr/bin/env bash
# The red stub of SPEC-480 R16's Timeouts by package: it keeps its input and prints a header alone,
# so its criterion fails by assertion until the script itself replaces this stub.
# usage: mutation-timeouts.sh <the verdict job's reports directory>
set -u
reports=$1
echo "Timeouts by package, over the leg reports under $reports"
exit 0
