#!/usr/bin/env python3
"""Static QA lint for Vela OS 2 components (no VM needed).

Checks (IDs from TEST_PLAN.md):
  R7/P-IT-2  banned marketing/placeholder phrases in user-facing files
  P-IT-1     likely English UI strings in first-party Vela UI (heuristic -> WARN)
  P-IT-1     .desktop files without Italian Name (Name[it] or Italian Name)
  build      hook.chroot.sh / deploy.sh pass `sh -n`; packages.list names exist in trixie (apt-cache)
Usage: static_lint.py [repo_root] [--out results.json]
Exit 1 if any FAIL.
"""
import json, os, re, subprocess, sys

_script_dir = os.path.dirname(os.path.abspath(__file__))
_default_root = os.path.normpath(os.path.join(_script_dir, '..', '..', '..'))
root = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith('--') else _default_root
out = sys.argv[sys.argv.index('--out') + 1] if '--out' in sys.argv else None
here = os.path.dirname(os.path.abspath(__file__))
banned = [re.compile(l.strip(), re.I) for l in open(os.path.join(here, '..', 'lib', 'banned_phrases.txt'))
          if l.strip() and not l.startswith('#')]
UI_EXT = {'.qml', '.desktop', '.ui', '.html', '.js', '.json', '.py', '.md', '.txt', '.conf', '.notifyrc', '.po', '.xml', '.sh'}
# Words that are almost never Italian; whole-word, in quoted UI strings only
EN = re.compile(r"\b(the|and|your|settings|search|open|close|cancel|save|delete|loading|welcome|click|please|error|success|done|next|back|skip|enable|disable|install|remove|update|apply|privacy|assistant|terminal|browser|home|files?|choose|select|start|get started|learn more|try again|something went wrong)\b", re.I)
UISTR = [re.compile(r'qsTr\(\s*"([^"]+)"'), re.compile(r'\btext\s*:\s*"([^"]+)"'), re.compile(r'\btitle\s*:\s*"([^"]+)"'),
         re.compile(r'placeholderText\s*:\s*"([^"]+)"'), re.compile(r'\b_\(\s*["\']([^"\']+)["\']'),
         re.compile(r'set(?:Text|WindowTitle|PlaceholderText|ToolTip)\(\s*(?:self\.tr\()?["\']([^"\']+)["\']')]
results = []
def rec(status, tid, msg): results.append({'status': status, 'id': tid, 'msg': msg})

comp_dir = os.path.join(root, 'components')
files_scanned = 0
for comp in sorted(os.listdir(comp_dir)):
    if comp == 'qa':
        continue
    cdir = os.path.join(comp_dir, comp)
    for dp, dn, fn in os.walk(cdir):
        dn[:] = [d for d in dn if d not in ('.git', 'node_modules', '__pycache__', 'vendor', 'tests', 'test')]
        for f in fn:
            p = os.path.join(dp, f); rel = os.path.relpath(p, root)
            ext = os.path.splitext(f)[1].lower()
            if ext not in UI_EXT and f not in ('hook.chroot.sh', 'deploy.sh'):
                continue
            if os.path.getsize(p) > 2_000_000:
                continue
            try:
                txt = open(p, encoding='utf-8', errors='replace').read()
            except Exception:
                continue
            files_scanned += 1
            is_doc = ext == '.md' and 'overlay' not in rel  # component docs are not UI
            if not is_doc:
                for i, line in enumerate(txt.splitlines(), 1):
                    for b in banned:
                        if b.search(line):
                            # placeholderText as a QML property name is fine; only flag the phrase in strings/text
                            if b.pattern == 'placeholder' and re.search(r'placeholder(Text|_text|Color)', line, re.I):
                                continue
                            if b.pattern in (r'\bTODO\b', r'\bFIXME\b', r'\bXXX\b') and ext in ('.sh', '.py') and line.lstrip().startswith('#'):
                                rec('WARN', 'R7', f'{rel}:{i}: code comment {b.pattern}: {line.strip()[:100]}')
                                continue
                            rec('FAIL', 'R7', f'{rel}:{i}: banned phrase /{b.pattern}/: {line.strip()[:120]}')
            if ext in ('.qml', '.py', '.ui', '.html', '.js'):
                for i, line in enumerate(txt.splitlines(), 1):
                    for rx in UISTR:
                        for m in rx.finditer(line):
                            s = m.group(1)
                            if EN.search(s) and not re.search(r'[àèéìòù]', s) and len(s) > 2:
                                rec('WARN', 'P-IT-1', f'{rel}:{i}: possible English UI string: "{s[:80]}"')
            if ext == '.desktop' and 'overlay' in rel and f != 'metadata.desktop' and '/servicemenus/' not in rel:
                if not re.search(r'^\[Desktop Entry\]', txt, re.M):
                    continue
                if re.search(r'^NoDisplay=true', txt, re.M) or re.search(r'^Hidden=true', txt, re.M):
                    continue
                has_it = re.search(r'^Name\[it\]=', txt, re.M)
                name = re.search(r'^Name=(.*)$', txt, re.M)
                if not has_it:
                    rec('WARN', 'P-IT-1', f'{rel}: no Name[it] (Name={name.group(1) if name else "?"}) — fine only if Name is already Italian/brand')
                if not re.search(r'^Comment\[it\]=|^GenericName\[it\]=', txt, re.M) and re.search(r'^Comment=', txt, re.M):
                    rec('WARN', 'P-IT-1', f'{rel}: Comment without Comment[it]')
                try:
                    r = subprocess.run(['desktop-file-validate', p], capture_output=True, text=True, timeout=20)
                    errs = [l for l in (r.stdout + r.stderr).splitlines() if 'error:' in l]
                    for e in errs:
                        rec('FAIL', 'desktop-file', f'{rel}: {e.split("error:",1)[1].strip()[:140]}')
                except FileNotFoundError:
                    pass
            if f in ('hook.chroot.sh', 'deploy.sh') or (ext == '.sh' and 'overlay' not in rel):
                r = subprocess.run(['sh', '-n', p], capture_output=True, text=True)
                rec('PASS' if r.returncode == 0 else 'FAIL', 'build-sh', f'{rel}: sh -n ' + ('ok' if r.returncode == 0 else r.stderr.strip()[:200]))
    pl = os.path.join(cdir, 'packages.list')
    if os.path.exists(pl):
        names = [l.split('#')[0].strip() for l in open(pl) if l.split('#')[0].strip()]
        missing = []
        for n in names:
            r = subprocess.run(['apt-cache', 'policy', n], capture_output=True, text=True)
            cand = re.search(r'Candidate:\s*(\S+)', r.stdout)
            if not cand or cand.group(1) == '(none)':
                # virtual packages: check showpkg reverse provides
                r2 = subprocess.run(['apt-cache', 'showpkg', n], capture_output=True, text=True)
                prov = r2.stdout.split('Reverse Provides:')[-1].strip() if 'Reverse Provides:' in r2.stdout else ''
                if not prov:
                    missing.append(n)
        FASTTRACK = {'virtualbox-guest-utils', 'virtualbox-guest-x11', 'virtualbox-guest-dkms'}
        ft = [m for m in missing if m in FASTTRACK]; missing = [m for m in missing if m not in FASTTRACK]
        if ft: rec('WARN', 'packages', f'components/{comp}/packages.list: from fasttrack repo (not trixie main), OK only if base adds fasttrack: {" ".join(ft)}')
        rec('FAIL' if missing else 'PASS', 'packages', f'components/{comp}/packages.list: {len(names)} pkgs' + (f', NOT in trixie: {" ".join(missing)}' if missing else ', all exist'))

rec('INFO', 'static', f'{files_scanned} files scanned')
for r in results:
    print(f"{r['status']:5} {r['id']:12} {r['msg']}")
if out:
    json.dump(results, open(out, 'w'), indent=1, ensure_ascii=False)
sys.exit(1 if any(r['status'] == 'FAIL' for r in results) else 0)
