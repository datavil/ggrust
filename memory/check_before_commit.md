# Never let a failing check reach a commit

**Broke:** `make check 2>&1 | grep ...; git add -A && git commit ...` committed and pushed a
lint failure (Phase 0.7): the pipe hid make's exit status and `;` ran the commit anyway.

**Works:** gate the commit on the check's own status:
`make check > /tmp/check.log 2>&1 && git add -A && git commit ...` (then inspect the log), or
`set -o pipefail`. If a bad commit slipped out, fix forward in a new commit; say so.
