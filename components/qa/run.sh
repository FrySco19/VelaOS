#!/bin/bash
# Run the QA suite.  usage: components/qa/run.sh <build-name> [static|system|complaints|parity|all]
# Needs for VM suites: QA_QMP (QMP socket) and guest SSH (QA_SSH_PORT, default 2222). See lib/qa.sh.
set -u
export QA_BUILD=${1:?build name}; what=${2:-all}
D=$(cd "$(dirname "$0")" && pwd)
. "$D/lib/qa.sh"
echo "evidence: $QA_OUT"
case $what in static|all) python3 "$D/tests/static_lint.py" "$QA_REPO" --out "$QA_OUT/static.json" | tee "$QA_OUT/static.log"
  awk '{s=$1; id=$2; $1=$2=""; printf "%s\t%s\t%s\t\n", s, id, substr($0,3)}' "$QA_OUT/static.log" >> "$QA_RESULTS";; esac
for t in system complaints parity; do
  case $what in $t|all) bash "$D/tests/t_$t.sh" 2>&1 | tee "$QA_OUT/t_$t.log";; esac
done
python3 - "$QA_RESULTS" "$QA_OUT/results.md" <<'PY'
import sys, collections
rows=[l.rstrip('\n').split('\t') for l in open(sys.argv[1]) if l.strip()]
c=collections.Counter(r[0] for r in rows)
with open(sys.argv[2],'w') as o:
    o.write(f"# QA results\n\n{dict(c)}\n\n| status | id | message | evidence |\n|---|---|---|---|\n")
    for r in rows: o.write('| ' + ' | '.join((r+['','','',''])[:4]).replace('\n',' ') + ' |\n')
print(dict(c))
PY
