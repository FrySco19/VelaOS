#!/usr/bin/env python3
"""Check shell syntax with the script's declared shell, without executing it."""
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
failed = []
for directory in ('build', 'tools', 'components', 'archive'):
    for script in sorted((root / directory).rglob('*.sh')):
        first = script.read_text(encoding='utf-8').splitlines()[0]
        shell = 'bash' if 'bash' in first else 'sh'
        if subprocess.run([shell, '-n', str(script)]).returncode:
            failed.append(str(script.relative_to(root)))
if failed:
    print('Invalid shell scripts:', ', '.join(failed), file=sys.stderr)
    sys.exit(1)
print('Shell syntax checks passed')
