"""Danger classifier for shell commands (Vela AI Terminal + safe executor).

classify(command) -> Assessment with level:
  "safe"    read-only, known command; still confirmed by the UI when run by the AI
  "change"  modifies files/settings/network state, or is unknown/unanalysable
  "danger"  destructive, irreversible, privileged or executes untrusted code:
            ALWAYS requires an explicit, separate confirmation (hard guard).

The classifier is conservative: anything it does not understand is at least
"change". It never executes anything.
"""
from __future__ import annotations

from dataclasses import dataclass, field
import posixpath
import re

from .shparse import parse, SimpleCommand, Word

SAFE, CHANGE, DANGER = "safe", "change", "danger"
_ORDER = {SAFE: 0, CHANGE: 1, DANGER: 2}


@dataclass
class Reason:
    level: str
    code: str
    it: str
    en: str

    def text(self, lang="it"):
        return self.it if lang == "it" else self.en


@dataclass
class Assessment:
    command: str
    level: str = SAFE
    reasons: list = field(default_factory=list)
    programs: list = field(default_factory=list)
    uncertain: bool = False

    @property
    def is_danger(self):
        return self.level == DANGER

    def add(self, level, code, it, en):
        if not any(r.code == code and r.it == it for r in self.reasons):
            self.reasons.append(Reason(level, code, it, en))
        if _ORDER[level] > _ORDER[self.level]:
            self.level = level

    def summary(self, lang="it"):
        top = [r for r in self.reasons if r.level == self.level] or self.reasons
        return "; ".join(r.text(lang) for r in top[:3])

    def to_dict(self, lang="it"):
        return {
            "command": self.command,
            "level": self.level,
            "requires_explicit_confirmation": self.level == DANGER,
            "reasons": [{"level": r.level, "code": r.code, "text": r.text(lang),
                         "it": r.it, "en": r.en} for r in self.reasons],
            "programs": self.programs,
            "uncertain": self.uncertain,
            "summary": self.summary(lang),
        }


# --------------------------------------------------------------------------
# Tables
# --------------------------------------------------------------------------
SHELLS = {"sh", "bash", "dash", "zsh", "ksh", "mksh", "fish", "csh", "tcsh", "busybox"}
INTERPRETERS = SHELLS | {"python", "python3", "python2", "perl", "ruby", "node", "nodejs",
                         "php", "lua", "tclsh", "pwsh", "deno", "bun"}
DOWNLOADERS = {"curl", "wget", "fetch", "aria2c", "http", "https", "xh", "lynx", "w3m",
               "nc", "ncat", "netcat", "socat"}
PRIV = {"sudo", "doas", "pkexec", "su", "run0", "sudoedit", "gksudo", "kdesu", "kdesudo"}
WRAPPERS_SIMPLE = {"nohup", "command", "builtin", "exec", "unbuffer", "setsid", "torsocks",
                   "proxychains", "proxychains4", "catchsegv", "valgrind", "strace", "ltrace",
                   "firejail", "dbus-run-session", "xvfb-run", "noglob", "chronic", "time"}
DISK_TOOLS = {"mkfs", "mke2fs", "mkswap", "mkdosfs", "mkntfs", "wipefs", "fdisk", "sfdisk",
              "gdisk", "sgdisk", "cgdisk", "parted", "cfdisk", "blkdiscard", "shred",
              "cryptsetup", "zpool", "zfs", "lvremove", "vgremove", "pvremove", "lvreduce",
              "mdadm", "badblocks", "hdparm", "nvme", "sg_format", "resize2fs", "e2fsck",
              "fsck", "tune2fs", "btrfs", "losetup", "dmsetup", "grub-install", "update-grub",
              "efibootmgr", "flashrom", "partprobe", "kpartx", "sfill", "srm", "secure-delete"}
POWER = {"shutdown", "reboot", "poweroff", "halt", "telinit", "init", "kexec"}
USER_ADMIN = {"userdel", "deluser", "useradd", "adduser", "usermod", "passwd", "chpasswd",
              "groupdel", "delgroup", "gpasswd", "chage", "visudo", "vipw", "chsh"}
FIREWALL = {"iptables", "ip6tables", "nft", "ufw", "firewall-cmd", "ebtables", "arptables"}
SYSTEM_DIRS = ("/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/libx32", "/opt",
               "/proc", "/root", "/run", "/sbin", "/srv", "/sys", "/usr", "/var", "/efi",
               "/snap", "/nix")
SAFE_DEVICES = {"/dev/null", "/dev/stdout", "/dev/stderr", "/dev/stdin", "/dev/tty",
                "/dev/zero", "/dev/random", "/dev/urandom", "/dev/full", "/dev/fd/1", "/dev/fd/2"}
BLOCKDEV_RE = re.compile(r"^/dev/(sd[a-z]+\d*|hd[a-z]+\d*|vd[a-z]+\d*|xvd[a-z]+\d*|nvme\d+(n\d+)?(p\d+)?|"
                         r"mmcblk\d+(p\d+)?|loop\d+|dm-\d+|md\d+|sr\d+|disk/.*|mapper/.*|mem|kmem|port|"
                         r"nbd\d+|zram\d+|ram\d+|cdrom|dvd|fb\d+|input/.*|bus/.*|mtd.*)$")
HOME_RC = (".bashrc", ".profile", ".bash_profile", ".zshrc", ".config/fish/config.fish",
           ".ssh/authorized_keys", ".ssh/config", ".ssh", ".gnupg", ".xprofile", ".pam_environment",
           ".config/autostart", ".config", ".local", ".local/share")

READONLY = {
    "ls", "ll", "la", "dir", "vdir", "pwd", "cd", "echo", "printf", "cat", "tac", "less", "more",
    "most", "head", "tail", "grep", "egrep", "fgrep", "rg", "ag", "ack", "du", "df", "free",
    "uptime", "whoami", "id", "groups", "date", "cal", "ncal", "uname", "hostname",
    "which", "type", "whereis", "file", "stat", "wc", "sort", "uniq", "cut", "paste", "join",
    "column", "fold", "fmt", "nl", "expand", "unexpand", "diff", "cmp", "comm", "colordiff",
    "tree", "ps", "pstree", "top", "htop", "btop", "atop", "lsblk", "lscpu", "lsusb",
    "lspci", "lsmod", "lshw", "lsof", "lsns", "lslocks", "lsipc", "blkid", "findmnt", "ss",
    "netstat", "ping", "ping6", "traceroute", "tracepath", "mtr", "dig", "nslookup", "host",
    "whois", "printenv", "man", "info", "help", "apropos", "whatis", "tldr",
    "nproc", "sensors", "realpath", "basename", "dirname", "readlink", "md5sum", "sha1sum",
    "sha256sum", "sha512sum", "b2sum", "cksum", "jq", "yq", "true", "false", "test", "[", "[[",
    "sleep", "seq", "yes", "expr", "bc", "dc", "factor", "numfmt", "locale",
    "w", "who", "last", "lastlog", "users", "vmstat", "iostat", "mpstat", "pidstat",
    "glxinfo", "inxi", "neofetch", "fastfetch", "screenfetch", "wmctrl", "xprop", "xwininfo",
    "pw-cli", "pw-top", "iwconfig", "getent", "fc-list", "fc-match", "lsattr", "getfacl",
    "namei", "pgrep", "pidof", "jobs", "fg", "bg", "wait", "unalias", "local", "declare",
    "read", "shift", "clear", "reset", "tput", "stty", "tty", "bat", "batcat", "eza", "exa",
    "fd", "fdfind", "fzf", "identify", "exiftool", "ffprobe", "mediainfo", "pdfinfo",
    "strings", "hexdump", "od", "xxd", "nm", "objdump", "readelf", "ldd", "zipinfo",
    "systemd-analyze", "coredumpctl", "acpi", "kreadconfig6", "kreadconfig5", "vainfo",
    "vulkaninfo", "nvidia-smi", "apt-cache", "dpkg-query", "base64", "sed", "awk", "gawk",
    "mawk", "set", "unset", "alias", "env", "history", "journalctl", "dmesg", "export",
    "xargs", "watch", "timeout", "nice", "ip", "nmcli", "pactl", "wpctl", "xrandr", "upower",
    "powerprofilesctl", "timedatectl", "localectl", "hostnamectl", "loginctl", "iw", "ethtool",
    "arp", "route", "gsettings", "qdbus", "qdbus6", "busctl", "vela-ai-config", "openssl",
    "lsb_release", "dpkg-architecture", "tr", "rev", "shuf", "tsort", "look", "iconv", "pr", "csplit_dummy", "ulimit", "umask", "hash", "exit", "return", "break", "continue", ":", "logout", "disown", "trap", "caller", "shopt", "complete", "compgen", "printf", "printf", "pinky", "tload",
}
KNOWN_CHANGE = {"tee", "install", "code", "nano", "vim", "vi", "nvim", "emacs", "kate", "micro",
                "make", "cmake", "gcc", "g++", "cc", "clang", "go", "rustc", "javac", "java",
                "konsole", "dolphin", "firefox", "chromium", "zip", "gzip", "gunzip", "xz",
                "bzip2", "zstd", "7z", "ssh-keygen", "gpg", "cargo", "kioclient", "kioclient5",
                "kioclient6", "ffmpeg", "convert", "magick", "pandoc", "notify-send", "kdialog"}


def _base(prog: str) -> str:
    p = prog.rsplit("/", 1)[-1]
    return p[1:] if p.startswith("\\") else p


def _norm_path(p: str) -> str:
    """Normalise a path argument for checks (keeps ~ symbolic)."""
    if not p:
        return p
    q = p.replace("${HOME}", "~").replace("$HOME", "~")
    if q.startswith("~") and not q.startswith("~/") and q != "~":
        return q  # ~user
    if q.startswith("/"):
        q = posixpath.normpath(q)
        if q.startswith("//"):
            q = "/" + q.lstrip("/")
    elif q.startswith("~/"):
        rest = posixpath.normpath(q[2:])
        q = "~" if rest in (".", "") else "~/" + rest
    return q


def is_root_or_home(p: str) -> bool:
    q = _norm_path(p.rstrip("/") or "/")
    return q in ("/", "/*", "~", "~/*", "~/.*", "/home", "/home/*", ".", "..", "./*", "*", ".*",
                 "../*", "/.*", "../..") or bool(re.fullmatch(r"/home/[^/]+/?\*?", q)) \
        or bool(re.fullmatch(r"~[a-z_][a-z0-9_-]*/?\*?", q))


def is_system_path(p: str) -> bool:
    q = _norm_path(p)
    if not q.startswith("/"):
        return False
    if q == "/":
        return True
    if q == "/tmp" or q.startswith(("/tmp/", "/var/tmp/", "/media/", "/mnt/", "/run/media/", "/run/user/")):
        return False
    if q.startswith("/home"):
        return bool(re.fullmatch(r"/home(/[^/]+)?/?", q))  # /home or a whole home dir
    return any(q == d or q.startswith(d + "/") for d in SYSTEM_DIRS)


def is_block_device(p: str) -> bool:
    q = _norm_path(p)
    return q not in SAFE_DEVICES and bool(BLOCKDEV_RE.match(q))


def touches_rc(p: str) -> bool:
    q = _norm_path(p).rstrip("/")
    for rc in HOME_RC:
        if q == "~/" + rc or q.endswith("/" + rc) and q.startswith(("/home/", "/root")) or q == rc:
            return True
    return False


def _flags(args):
    """Return (set of single-letter flags, set of long flags, positional args)."""
    short, long_, pos = set(), set(), []
    end = False
    for a in args:
        if end:
            pos.append(a)
        elif a == "--":
            end = True
        elif a.startswith("--"):
            long_.add(a.split("=", 1)[0])
        elif a.startswith("-") and len(a) > 1 and not a[1:2].isdigit():
            short.update(a[1:])
        else:
            pos.append(a)
    return short, long_, pos


ASSIGN_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\[[^]]*\])?\+?=")
SENSITIVE_VARS = ("LD_PRELOAD", "LD_LIBRARY_PATH", "PATH", "PROMPT_COMMAND", "BASH_ENV",
                  "ENV", "IFS", "PYTHONPATH")


class _Analyzer:
    def __init__(self, a: Assessment, depth=0):
        self.a = a
        self.depth = depth

    def nested(self, text: str):
        if self.depth > 6:
            self.a.add(DANGER, "too_deep", "comando troppo annidato per essere analizzato",
                       "command nested too deeply to analyse")
            return
        sub = Assessment(text)
        _analyze(text, sub, self.depth + 1)
        for r in sub.reasons:
            self.a.add(r.level, r.code, r.it, r.en)
        self.a.programs += [p for p in sub.programs if p not in self.a.programs]
        self.a.uncertain = self.a.uncertain or sub.uncertain

    def command(self, cmd: SimpleCommand, words=None):
        a = self.a
        words = list(cmd.words if words is None else words)
        while words and ASSIGN_RE.match(words[0].value):
            name = words[0].value.split("=", 1)[0]
            if name in SENSITIVE_VARS:
                a.add(CHANGE, "env_sensitive", f"modifica la variabile sensibile {name}",
                      f"sets sensitive variable {name}")
            words = words[1:]
        while words and words[0].value in ("{", "}", "!", "then", "do", "else", "elif", "if",
                                           "while", "until", "fi", "done", "esac", "coproc",
                                           "function"):
            words = words[1:]
        if not words:
            return
        w0 = words[0]
        if w0.value in ("for", "select", "case", "in"):
            return
        prog = _base(w0.value)
        args = [w.value for w in words[1:]]
        if w0.has_expansion and not w0.quoted:
            a.add(CHANGE, "dynamic_program", "il nome del programma è calcolato al momento",
                  "program name is computed at runtime")
            a.uncertain = True
        if prog and prog not in a.programs:
            a.programs.append(prog)

        # ---- privilege escalation --------------------------------------
        if prog in PRIV:
            a.add(DANGER, "privilege", f"usa {prog}: esegue come amministratore (root)",
                  f"uses {prog}: runs as administrator (root)")
            rest = words[1:]
            if prog == "su":
                for i, w in enumerate(rest):
                    if w.value in ("-c", "--command") and i + 1 < len(rest):
                        self.nested(rest[i + 1].value)
                return
            i = 0
            opts_with_arg = {"-u", "-g", "-h", "-p", "-C", "-D", "-r", "-t", "-U", "--user",
                             "--group", "--host", "--prompt", "--chdir"}
            while i < len(rest) and rest[i].value.startswith("-"):
                if rest[i].value in opts_with_arg:
                    i += 1
                i += 1
            if i < len(rest):
                self.command(cmd, rest[i:])
            return
        # ---- transparent wrappers --------------------------------------
        if prog in WRAPPERS_SIMPLE or prog in ("env", "timeout", "watch", "xargs", "parallel",
                                                "nice", "ionice", "chrt", "taskset", "stdbuf",
                                                "chroot", "unshare", "nsenter", "script",
                                                "systemd-run", "flock", "caffeinate"):
            rest = words[1:]
            i = 0
            if prog == "env":
                while i < len(rest) and (rest[i].value.startswith("-") or "=" in rest[i].value):
                    if "=" in rest[i].value and rest[i].value.split("=", 1)[0] in SENSITIVE_VARS:
                        a.add(CHANGE, "env_sensitive", "modifica una variabile sensibile",
                              "sets a sensitive variable")
                    if rest[i].value in ("-u", "-C", "--unset", "--chdir"):
                        i += 1
                    if rest[i].value in ("-S", "--split-string") and i + 1 < len(rest):
                        self.nested(rest[i + 1].value)
                        return
                    i += 1
            elif prog == "timeout":
                while i < len(rest) and rest[i].value.startswith("-"):
                    if rest[i].value in ("-s", "-k", "--signal", "--kill-after"):
                        i += 1
                    i += 1
                i += 1  # duration
            elif prog == "watch":
                while i < len(rest) and rest[i].value.startswith("-"):
                    if rest[i].value in ("-n", "--interval"):
                        i += 1
                    i += 1
                if len(rest) - i == 1:
                    self.nested(rest[i].value)
                    return
            elif prog in ("xargs", "parallel"):
                opts_with_arg = {"-I", "-i", "-n", "-P", "-L", "-d", "-E", "-s", "-a", "-j"}
                while i < len(rest) and rest[i].value.startswith("-"):
                    if rest[i].value in opts_with_arg:
                        i += 1
                    i += 1
                if i >= len(rest):
                    return
            elif prog == "chroot":
                a.add(DANGER, "chroot", "chroot cambia la radice del sistema", "chroot changes the system root")
                i = 1
            elif prog == "script":
                for j, w in enumerate(rest):
                    if w.value in ("-c", "--command") and j + 1 < len(rest):
                        self.nested(rest[j + 1].value)
                return
            elif prog in ("nice", "ionice", "chrt", "taskset", "stdbuf"):
                while i < len(rest) and rest[i].value.startswith("-"):
                    if rest[i].value in ("-n", "-c", "-p", "--adjustment", "--class"):
                        i += 1
                    i += 1
                if prog in ("chrt", "taskset") and i < len(rest):
                    i += 1
            else:
                while i < len(rest) and rest[i].value.startswith("-"):
                    i += 1
                if prog == "flock" and i < len(rest):
                    i += 1
            if i < len(rest):
                self.command(cmd, rest[i:])
            return

        # ---- shells and interpreters --------------------------------------
        if prog in SHELLS:
            for i, w in enumerate(words[1:]):
                v = w.value
                if v == "-c" or (v.startswith("-") and not v.startswith("--") and "c" in v[1:]):
                    if i + 2 < len(words):
                        self.nested(words[i + 2].value)
                        return
            rest = [w for w in words[1:] if not w.value.startswith("-")]
            if any(w.value == "<(...)" for w in rest):
                a.add(DANGER, "exec_procsub", f"{prog} esegue codice prodotto da un altro comando",
                      f"{prog} runs code produced by another command")
            elif rest:
                a.add(CHANGE, "script", f"esegue lo script {rest[0].value}: il contenuto non è verificato",
                      f"runs script {rest[0].value}: content not verified")
            else:
                a.add(CHANGE, "subshell", "apre una nuova shell", "starts a new shell")
            return
        if prog in INTERPRETERS or re.fullmatch(r"python[23]?(\.\d+)?", prog):
            self.interpreter(prog, words, args)
            return
        if prog == "eval":
            self.nested(" ".join(args))
            a.add(CHANGE, "eval", "eval esegue testo come comando", "eval runs text as a command")
            a.uncertain = True
            return
        if prog in ("source", "."):
            a.add(CHANGE, "source", "carica ed esegue un file nella shell corrente",
                  "loads and runs a file in the current shell")
            if args and (args[0].startswith("/dev/") or args[0] == "<(...)" or args[0] == "-"):
                a.add(DANGER, "source_stream", "esegue codice letto da un flusso",
                      "runs code read from a stream")
            return

        handler = getattr(self, "p_" + re.sub(r"[^a-z0-9]", "_", prog.lower()), None)
        if prog.startswith("mkfs") or prog in DISK_TOOLS:
            self.p_disktool(prog, args)
        elif handler:
            handler(prog, args, words)
        elif prog in POWER:
            a.add(DANGER, "power", "spegne o riavvia il computer", "shuts down or reboots the computer")
        elif prog in USER_ADMIN:
            a.add(DANGER, "user_admin", "modifica utenti, password o permessi di sistema",
                  "changes users, passwords or system permissions")
        elif prog in FIREWALL:
            if any(x in args for x in ("-L", "--list", "list", "status", "-S")) and not any(
                    x in args for x in ("-F", "--flush", "flush", "-X", "-D", "delete", "disable", "reset")):
                pass
            else:
                a.add(DANGER, "firewall", "modifica il firewall", "changes the firewall")
        elif prog in READONLY:
            pass
        elif prog in KNOWN_CHANGE:
            a.add(CHANGE, "modifies", f"{prog} può creare o modificare file",
                  f"{prog} may create or modify files")
        elif prog.endswith(".sh") or "/" in w0.value:
            a.add(CHANGE, "script", f"esegue {w0.value}: il contenuto non è verificato",
                  f"runs {w0.value}: content not verified")
        else:
            a.add(CHANGE, "unknown", f"comando non riconosciuto ({prog}): controllalo prima di eseguirlo",
                  f"unrecognised command ({prog}): review before running")

    def interpreter(self, prog, words, args):
        a = self.a
        code = None
        for i, w in enumerate(words[1:]):
            if w.value in ("-c", "-e", "-E", "--eval", "-r") and i + 2 < len(words):
                code = words[i + 2].value
                break
        if code is not None:
            if re.search(r"rmtree|os\.remove|unlink|os\.system|subprocess|shutil\.|"
                         r"system\(|exec\(|rm -rf|child_process|fs\.rm|File\.delete|os\.rmdir|"
                         r"Path\(.*\)\.unlink|remove\(", code):
                a.add(DANGER, "inline_code_destructive",
                      "codice inline che cancella file o lancia altri comandi",
                      "inline code that deletes files or spawns commands")
            else:
                a.add(CHANGE, "inline_code", "esegue codice inline non analizzabile",
                      "runs inline code that cannot be analysed")
            return
        if args and args[0] in ("--version", "-V", "--help", "-h", "-v"):
            return
        if prog.startswith("python") and len(args) >= 2 and args[0] == "-m":
            mod = args[1]
            if mod == "http.server":
                a.add(CHANGE, "server", "avvia un server web locale", "starts a local web server")
                return
            if mod == "pip":
                self.p_pip("pip", args[2:], words)
                return
            if mod in ("json.tool", "this", "platform", "site"):
                return
        if [x for x in args if not x.startswith("-")]:
            a.add(CHANGE, "script", f"esegue un programma {prog}: il contenuto non è verificato",
                  f"runs a {prog} program: content not verified")
        else:
            a.add(CHANGE, "repl", f"apre l'interprete {prog}", f"opens the {prog} interpreter")

    # ------------------------------------------------------------------ files
    def p_disktool(self, prog, args):
        if prog == "btrfs" and len(args) >= 2 and args[0] in ("filesystem", "fi") and args[1] in ("show", "df", "usage", "du"):
            return
        if prog == "btrfs" and len(args) >= 2 and args[0] in ("subvolume", "su") and args[1] in ("list", "show"):
            return
        if prog == "zfs" and args[:1] and args[0] in ("list", "get"):
            return
        if prog == "zpool" and args[:1] and args[0] in ("list", "status", "get", "iostat"):
            return
        if prog in ("fdisk", "sfdisk") and any(x in ("-l", "--list", "-d", "--dump") for x in args):
            return
        if prog == "parted" and ("print" in args or "-l" in args):
            return
        if prog == "losetup" and (not args or args[0] in ("-l", "-a", "--list")):
            return
        if prog == "efibootmgr" and (not args or args == ["-v"]):
            return
        if prog == "cryptsetup" and args[:1] and args[0] in ("status", "luksDump", "isLuks"):
            return
        if prog == "mdadm" and args[:1] and args[0] in ("--detail", "-D", "--examine", "-E"):
            return
        if prog in ("e2fsck", "fsck") and "-n" in args:
            return
        if prog in ("shred", "srm", "sfill", "secure-delete"):
            self.a.add(DANGER, "shred", "distrugge i file in modo irrecuperabile", "destroys files irrecoverably")
            return
        self.a.add(DANGER, "disk", f"{prog} modifica dischi o partizioni: può cancellare tutti i dati",
                   f"{prog} changes disks or partitions: can erase all data")

    def p_rm(self, prog, args, words):
        a = self.a
        short, long_, pos = _flags(args)
        recursive = bool({"r", "R"} & short) or "--recursive" in long_
        force = "f" in short or "--force" in long_
        globby = any(w.has_glob and not w.quoted for w in words[1:])
        if "--no-preserve-root" in long_:
            a.add(DANGER, "rm_root", "rm --no-preserve-root può cancellare l'intero sistema",
                  "rm --no-preserve-root can delete the whole system")
        for p in pos:
            if is_root_or_home(p):
                a.add(DANGER, "rm_critical", f"cancella {p}: la home, la radice o un'intera cartella",
                      f"deletes {p}: home, root or an entire directory")
            elif is_system_path(p):
                a.add(DANGER, "rm_system", f"cancella file di sistema ({p})", f"deletes system files ({p})")
            elif touches_rc(p):
                a.add(DANGER, "rm_config", f"cancella una configurazione importante ({p})",
                      f"deletes important configuration ({p})")
        if recursive and force:
            a.add(DANGER, "rm_rf", "rm -rf cancella cartelle intere senza chiedere e senza cestino",
                  "rm -rf deletes whole directories without asking and without trash")
        elif recursive:
            a.add(DANGER, "rm_r", "cancella cartelle intere in modo definitivo (senza cestino)",
                  "deletes whole directories permanently (no trash)")
        elif force and globby:
            a.add(DANGER, "rm_f_glob", "cancella in blocco con un carattere jolly, senza conferme",
                  "bulk delete with a wildcard, without prompts")
        elif any(w.has_expansion and not w.quoted for w in words[1:]):
            a.add(DANGER, "rm_dynamic", "cancella file il cui nome è calcolato al momento",
                  "deletes files whose names are computed at runtime")
        else:
            a.add(CHANGE, "rm", "cancella file in modo definitivo (senza cestino)",
                  "deletes files permanently (no trash)")

    p_unlink = p_rm

    def p_rmdir(self, prog, args, words):
        _s, _l, pos = _flags(args)
        if any(is_system_path(p) or is_root_or_home(p) for p in pos):
            self.a.add(DANGER, "rmdir_system", "rimuove cartelle di sistema", "removes system directories")
        else:
            self.a.add(CHANGE, "rmdir", "rimuove cartelle vuote", "removes empty directories")

    def p_dd(self, prog, args, words):
        of = [x[3:] for x in args if x.startswith("of=")]
        if of and (is_block_device(of[0]) or is_system_path(of[0])):
            self.a.add(DANGER, "dd_device", f"dd scrive direttamente su {of[0]}: può cancellare un disco",
                       f"dd writes directly to {of[0]}: can wipe a disk")
        elif of:
            self.a.add(DANGER, "dd", "dd sovrascrive dati byte per byte senza conferme",
                       "dd overwrites data byte by byte without prompts")
        else:
            self.a.add(CHANGE, "dd_read", "dd legge dati grezzi", "dd reads raw data")

    def _perm(self, prog, args, what_it, what_en):
        a = self.a
        short, long_, pos = _flags(args)
        recursive = "R" in short or "--recursive" in long_
        targets = pos[1:] if pos else []
        mode = pos[0] if pos else ""
        crit = [t for t in targets if is_root_or_home(t) or is_system_path(t)]
        if prog == "chmod" and re.search(r"777|666|a\+[rwx]*w|o\+[rwx]*w|\+w", mode) and (recursive or crit):
            a.add(DANGER, "perm_world", "rende i file scrivibili da chiunque", "makes files writable by anyone")
        for t in crit:
            a.add(DANGER, "perm_system", f"{what_it} su {t} (sistema o intera home)",
                  f"{what_en} on {t} (system or whole home)")
        if recursive and prog != "chmod":
            a.add(DANGER, "perm_recursive", f"{what_it} in modo ricorsivo", f"{what_en} recursively")
        elif recursive:
            a.add(CHANGE, "perm_recursive", f"{what_it} in modo ricorsivo", f"{what_en} recursively")
        a.add(CHANGE, "perm", what_it, what_en)

    def p_chmod(self, prog, args, words):
        self._perm(prog, args, "cambia i permessi", "changes permissions")

    def p_chown(self, prog, args, words):
        self._perm(prog, args, "cambia il proprietario", "changes ownership")

    p_chgrp = p_chown

    def p_setfacl(self, prog, args, words):
        self._perm("chmod", ["x"] + args, "cambia i permessi ACL", "changes ACL permissions")

    def p_chattr(self, prog, args, words):
        self.a.add(DANGER, "chattr", "cambia gli attributi speciali dei file (es. immutabile)",
                   "changes special file attributes (e.g. immutable)")

    def _dest_check(self, prog, args, verb_it, verb_en):
        a = self.a
        _s, _l, pos = _flags(args)
        for p in pos:
            if p == "/dev/null" and prog == "mv":
                a.add(DANGER, "mv_null", "sposta file in /dev/null: vengono distrutti",
                      "moves files to /dev/null: they are destroyed")
            elif is_system_path(p) or is_block_device(p) or (prog == "mv" and is_root_or_home(p) and p not in (".", "..")):
                a.add(DANGER, f"{prog}_system", f"{verb_it} file di sistema o intere cartelle ({p})",
                      f"{verb_en} system files or whole folders ({p})")
            elif touches_rc(p):
                a.add(CHANGE, f"{prog}_rc", f"tocca una configurazione importante ({p})",
                      f"touches important configuration ({p})")
        a.add(CHANGE, prog, f"{verb_it} file (può sovrascrivere quelli esistenti)",
              f"{verb_en} files (may overwrite existing ones)")

    def p_mv(self, prog, args, words):
        self._dest_check(prog, args, "sposta o rinomina", "moves or renames")

    def p_cp(self, prog, args, words):
        self._dest_check(prog, args, "copia", "copies")

    def p_install(self, prog, args, words):
        self._dest_check(prog, args, "installa", "installs")

    def p_ln(self, prog, args, words):
        self._dest_check(prog, args, "crea collegamenti", "creates links")

    def p_rsync(self, prog, args, words):
        short, long_, pos = _flags(args)
        if any(l.startswith("--delete") for l in long_) or "--remove-source-files" in long_:
            self.a.add(DANGER, "rsync_delete", "rsync con --delete cancella file nella destinazione",
                       "rsync --delete removes files at the destination")
        if any(":" in p and not p.startswith("/") for p in pos):
            self.a.add(CHANGE, "network_send", "trasferisce file in rete", "transfers files over the network")
        if "-n" in args or "--dry-run" in args or ("n" in short):
            self.a.add(CHANGE, "rsync_dry", "rsync in prova (non modifica nulla)", "rsync dry run")
            return
        self._dest_check(prog, pos, "sincronizza", "syncs")

    def p_mkdir(self, prog, args, words):
        _s, _l, pos = _flags(args)
        if any(is_system_path(p) for p in pos):
            self.a.add(DANGER, "mkdir_system", "crea cartelle di sistema", "creates system directories")
        self.a.add(CHANGE, "mkdir", "crea cartelle", "creates folders")

    def p_touch(self, prog, args, words):
        _s, _l, pos = _flags(args)
        if any(is_system_path(p) for p in pos):
            self.a.add(DANGER, "touch_system", "crea o modifica file di sistema", "creates or changes system files")
        self.a.add(CHANGE, "touch", "crea file o ne aggiorna la data", "creates files or updates timestamps")

    def p_truncate(self, prog, args, words):
        _s, _l, pos = _flags(args)
        if any(is_system_path(p) or is_block_device(p) for p in pos):
            self.a.add(DANGER, "truncate_system", "svuota file di sistema", "empties system files")
        self.a.add(CHANGE, "truncate", "svuota o ridimensiona file", "empties or resizes files")

    def p_tee(self, prog, args, words):
        s, _l, pos = _flags(args)
        for p in pos:
            self.write_target(p, append="a" in s)

    def p_find(self, prog, args, words):
        a = self.a
        if "-delete" in args:
            a.add(DANGER, "find_delete", "find -delete cancella tutti i file trovati",
                  "find -delete removes every file it matches")
        for i, x in enumerate(args):
            if x in ("-exec", "-execdir", "-ok", "-okdir"):
                inner = []
                for y in args[i + 1:]:
                    if y in (";", "+", "\\;"):
                        break
                    inner.append(y)
                if inner:
                    self.command(SimpleCommand(words=[Word(value=v, raw=v) for v in inner]))
                    if _base(inner[0]) in ("rm", "shred", "unlink", "mv", "chmod", "chown", "dd", "truncate"):
                        a.add(DANGER, "find_exec_destructive",
                              f"applica {_base(inner[0])} a ogni file trovato da find",
                              f"applies {_base(inner[0])} to every file found")
            if x in ("-fprint", "-fprintf", "-fls", "-fprint0"):
                a.add(CHANGE, "find_write", "find scrive su un file", "find writes to a file")

    def p_sed(self, prog, args, words):
        inplace = any(x == "-i" or x.startswith("--in-place") or
                      (x.startswith("-") and not x.startswith("--") and "i" in x[1:]) for x in args)
        if inplace:
            _s, _l, pos = _flags(args)
            if any(is_system_path(p) for p in pos[1:]):
                self.a.add(DANGER, "sed_system", "modifica file di sistema sul posto", "edits system files in place")
            self.a.add(CHANGE, "sed_inplace", "modifica file sul posto (sed -i)", "edits files in place (sed -i)")
        script = " ".join(x for x in args if not x.startswith("-"))
        if re.search(r"(^|[;}\s])e(\s|$)|/e\b|(^|[;}\s/])w\s+\S", script):
            self.a.add(CHANGE, "sed_exec", "lo script sed può eseguire comandi o scrivere file",
                       "the sed script may run commands or write files")

    def p_awk(self, prog, args, words):
        txt = " ".join(args)
        if "system(" in txt or "| getline" in txt or re.search(r"print[^;]*>\s*\S", txt) or "inplace" in txt:
            self.a.add(CHANGE, "awk_side_effects", "lo script awk può eseguire comandi o scrivere file",
                       "the awk script may run commands or write files")

    p_gawk = p_awk
    p_mawk = p_awk

    def p_tar(self, prog, args, words):
        first = args[0].lstrip("-") if args and not args[0].startswith("--") else ""
        flags = first + "".join(x[1:] for x in args[1:] if x.startswith("-") and not x.startswith("--"))
        long_ = [x for x in args if x.startswith("--")]
        if ("t" in flags or "--list" in long_) and "x" not in flags and "c" not in flags:
            return
        if "x" in flags or "--extract" in long_ or "--get" in long_:
            if "-C" in args:
                idx = args.index("-C")
                if idx + 1 < len(args) and is_system_path(args[idx + 1]):
                    self.a.add(DANGER, "extract_system", "estrae file in una cartella di sistema",
                               "extracts files into a system directory")
            self.a.add(CHANGE, "extract", "estrae file (può sovrascrivere quelli esistenti)",
                       "extracts files (may overwrite existing ones)")
        else:
            self.a.add(CHANGE, "archive", "crea o modifica un archivio", "creates or changes an archive")

    def p_unzip(self, prog, args, words):
        if any(x in ("-l", "-v", "-t", "-Z", "-lv") for x in args):
            return
        self.a.add(CHANGE, "extract", "estrae file (può sovrascrivere quelli esistenti)",
                   "extracts files (may overwrite existing ones)")

    def p_gio(self, prog, args, words):
        sub = args[0] if args else ""
        if sub == "trash":
            if "--empty" in args:
                self.a.add(DANGER, "trash_empty", "svuota il cestino in modo definitivo", "empties the trash permanently")
            else:
                self.a.add(CHANGE, "trash", "sposta file nel cestino (recuperabili)", "moves files to the trash (recoverable)")
        elif sub in ("list", "info", "cat", "tree", "mime", "help", "version", ""):
            return
        elif sub == "remove":
            self.a.add(CHANGE, "rm", "cancella file in modo definitivo", "deletes files permanently")
        elif sub == "open":
            self.a.add(CHANGE, "open", "apre un file o un programma", "opens a file or program")
        else:
            self.a.add(CHANGE, "gio", f"gio {sub} modifica file", f"gio {sub} changes files")

    def p_trash_put(self, prog, args, words):
        self.a.add(CHANGE, "trash", "sposta file nel cestino (recuperabili)", "moves files to the trash (recoverable)")

    p_trash = p_trash_put

    def p_trash_empty(self, prog, args, words):
        self.a.add(DANGER, "trash_empty", "svuota il cestino in modo definitivo", "empties the trash permanently")

    def p_xdg_open(self, prog, args, words):
        self.a.add(CHANGE, "open", "apre un file o un programma", "opens a file or program")

    # ------------------------------------------------------------------ processes / jobs
    def p_kill(self, prog, args, words):
        _s, _l, pos = _flags(args)
        if any(p in ("1", "-1") for p in pos) or args[-1:] == ["-1"]:
            self.a.add(DANGER, "kill_all", "termina tutti i processi o init: blocca la sessione",
                       "kills all processes or init: crashes the session")
        if args[:1] in (["-l"], ["-L"]):
            return
        self.a.add(CHANGE, "kill", "termina processi (il lavoro non salvato va perso)",
                   "terminates processes (unsaved work is lost)")

    def p_pkill(self, prog, args, words):
        _s, _l, pos = _flags(args)
        crit = {"systemd", "init", "kwin_wayland", "kwin_x11", "plasmashell", "sddm", "Xorg",
                "Xwayland", "sshd", "dbus-daemon", "pipewire", "NetworkManager", "bash", "fish",
                "zsh", "sh", "startplasma-wayland", "ksmserver", "dbus-broker"}
        if any(p in crit for p in pos) or "-u" in args or "-U" in args or "-9" in args and not pos:
            self.a.add(DANGER, "kill_session", "termina processi essenziali della sessione",
                       "kills processes the session depends on")
        self.a.add(CHANGE, "kill", "termina processi (il lavoro non salvato va perso)",
                   "terminates processes (unsaved work is lost)")

    p_killall = p_pkill
    p_xkill = p_pkill

    def p_crontab(self, prog, args, words):
        if "-r" in args:
            self.a.add(DANGER, "crontab_remove", "cancella tutte le attività pianificate", "deletes all scheduled jobs")
        elif args == ["-l"]:
            return
        else:
            self.a.add(CHANGE, "crontab", "modifica le attività pianificate", "changes scheduled jobs")

    def p_at(self, prog, args, words):
        self.a.add(CHANGE, "schedule", "pianifica un comando", "schedules a command")

    # ------------------------------------------------------------------ git
    def p_git(self, prog, args, words):
        a = self.a
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] in ("-C", "-c", "--git-dir", "--work-tree"):
                i += 1
            i += 1
        sub = args[i] if i < len(args) else ""
        rest = args[i + 1:]
        restpos = [x for x in rest if not x.startswith("-")]
        if sub in ("status", "log", "diff", "show", "blame", "shortlog", "describe", "rev-parse",
                   "ls-files", "ls-tree", "grep", "whatchanged", "cat-file", "help", "version",
                   "--version", "rev-list", "show-ref", "for-each-ref", "", "count-objects", "check-ignore"):
            return
        if sub == "reflog" and (not rest or rest[0] in ("show", "list")):
            return
        if sub == "reset" and any(x in ("--hard", "--merge", "--keep") for x in rest):
            a.add(DANGER, "git_reset_hard", "git reset --hard butta via le modifiche non salvate",
                  "git reset --hard discards uncommitted changes")
        elif sub == "clean":
            if any(x.startswith("-") and not x.startswith("--") and "n" in x for x in rest) or "--dry-run" in rest:
                return
            a.add(DANGER, "git_clean", "git clean cancella file non tracciati in modo definitivo",
                  "git clean permanently deletes untracked files")
        elif sub == "push" and any(x in ("-f", "--force", "--mirror", "--delete", "-d", "--prune")
                                   or x.startswith("--force") or x.startswith("+") or x.startswith(":")
                                   for x in rest):
            a.add(DANGER, "git_force_push", "push forzato o cancellazione: può distruggere lavoro sul server",
                  "force push or delete: can destroy work on the server")
        elif sub == "checkout" and ("-f" in rest or "--force" in rest or rest[:1] == ["--"] or "." in restpos):
            a.add(DANGER, "git_discard", "butta via le modifiche locali ai file", "discards local file changes")
        elif sub == "restore" and "--staged" not in rest:
            a.add(DANGER, "git_discard", "butta via le modifiche locali ai file", "discards local file changes")
        elif sub == "branch" and ("-D" in rest or ("--delete" in rest and "--force" in rest)):
            a.add(DANGER, "git_branch_delete", "cancella un ramo anche se non unito", "force-deletes a branch")
        elif sub == "stash" and rest[:1] and rest[0] in ("clear", "drop"):
            a.add(DANGER, "git_stash_drop", "cancella modifiche salvate con stash", "deletes stashed changes")
        elif sub in ("filter-branch", "filter-repo") or (sub == "gc" and any("prune" in x for x in rest)) or (
                sub == "reflog" and rest[:1] in (["expire"], ["delete"])):
            a.add(DANGER, "git_rewrite", "riscrive o elimina la cronologia del repository",
                  "rewrites or deletes repository history")
        elif sub == "config":
            if any(x in ("--get", "--list", "-l", "--get-all", "--get-regexp") for x in rest) or len(restpos) < 2:
                return
            a.add(CHANGE, "git_config", "cambia la configurazione di git", "changes git configuration")
        elif sub == "remote":
            if not restpos or restpos[0] in ("show", "get-url"):
                return
            a.add(CHANGE, "git", "modifica i remoti di git", "changes git remotes")
        elif sub in ("branch", "tag"):
            if not restpos and not any(x in ("-d", "--delete", "-m", "-M", "-c", "-C", "-f", "--force") for x in rest):
                return
            a.add(CHANGE, "git", f"modifica {'i rami' if sub == 'branch' else 'i tag'}", f"changes {sub}es")
        elif sub == "stash" and rest[:1] in (["list"], ["show"]):
            return
        else:
            if sub == "push":
                a.add(CHANGE, "network_send", "invia commit al server", "sends commits to the server")
            a.add(CHANGE, "git", f"git {sub} modifica il repository", f"git {sub} changes the repository")

    # ------------------------------------------------------------------ packages
    def _pkg(self, args, remove_words, install_words, ro_words):
        a = self.a
        sub = next((x for x in args if not x.startswith("-")), "")
        if any(x in ("-s", "--simulate", "--dry-run", "--just-print", "--no-act", "--assume-no") for x in args):
            return
        if sub in remove_words:
            a.add(DANGER, "pkg_remove", "rimuove programmi dal sistema", "removes software from the system")
        elif sub in install_words:
            a.add(CHANGE, "pkg_install", "installa o aggiorna programmi", "installs or upgrades software")
        elif sub in ro_words or not sub:
            return
        else:
            a.add(CHANGE, "pkg", "modifica i pacchetti installati", "changes installed packages")

    def p_apt(self, prog, args, words):
        self._pkg(args, {"remove", "purge", "autoremove", "autopurge"},
                  {"install", "upgrade", "full-upgrade", "dist-upgrade", "reinstall", "update",
                   "build-dep", "satisfy", "edit-sources", "clean", "autoclean", "modernize-sources"},
                  {"list", "search", "show", "policy", "depends", "rdepends", "changelog", "showsrc",
                   "help", "moo", "download", "source", "check", "indextargets"})

    p_apt_get = p_apt
    p_aptitude = p_apt
    p_nala = p_apt

    def p_dpkg(self, prog, args, words):
        if any(x in ("-r", "--remove", "-P", "--purge") or x.startswith("--force") for x in args):
            self.a.add(DANGER, "pkg_remove", "rimuove pacchetti di sistema (dpkg)", "removes system packages (dpkg)")
        elif any(x in ("-i", "--install", "--configure", "--unpack", "--set-selections", "-a") for x in args):
            self.a.add(CHANGE, "pkg_install", "installa pacchetti (dpkg)", "installs packages (dpkg)")

    def p_flatpak(self, prog, args, words):
        self._pkg(args, {"uninstall", "remove"},
                  {"install", "update", "upgrade", "repair", "remote-add", "remote-delete", "override",
                   "permission-set", "permission-reset", "remote-modify", "mask", "pin"},
                  {"list", "search", "info", "remotes", "remote-ls", "history", "ps", "permissions",
                   "permission-show", "documents", "--version", "remote-info"})
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub == "run":
            self.a.add(CHANGE, "open", "avvia un'applicazione", "starts an application")

    p_snap = p_flatpak

    def p_pip(self, prog, args, words):
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub in ("install", "uninstall", "download"):
            if "--break-system-packages" in args:
                self.a.add(DANGER, "pip_system", "pip modifica i pacchetti Python di sistema",
                           "pip modifies system Python packages")
            self.a.add(CHANGE, "pkg_user", "installa o rimuove pacchetti Python", "installs or removes Python packages")

    p_pip3 = p_pip
    p_pipx = p_pip

    def p_npm(self, prog, args, words):
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub in ("ls", "list", "view", "search", "outdated", "help", "-v", "--version", "", "info"):
            return
        self.a.add(CHANGE, "pkg_user", f"npm {sub} modifica il progetto o esegue script",
                   f"npm {sub} changes the project or runs scripts")

    p_yarn = p_npm
    p_pnpm = p_npm

    # ------------------------------------------------------------------ system / services
    def p_systemctl(self, prog, args, words):
        a = self.a
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub in ("status", "show", "list-units", "list-unit-files", "list-timers", "list-sockets",
                   "is-active", "is-enabled", "is-failed", "cat", "help", "list-dependencies",
                   "get-default", "list-jobs", "list-machines", ""):
            return
        if sub in ("poweroff", "reboot", "halt", "suspend", "hibernate", "hybrid-sleep", "kexec",
                   "emergency", "rescue", "isolate", "soft-reboot", "default", "suspend-then-hibernate"):
            a.add(DANGER, "power", "cambia lo stato del computer (spegne, riavvia, sospende)",
                  "changes machine state (power off, reboot, suspend)")
        elif "--user" in args:
            a.add(CHANGE, "service_user", f"systemctl --user {sub}: modifica un servizio dell'utente",
                  f"systemctl --user {sub}: changes a user service")
        else:
            a.add(DANGER, "service_system", f"systemctl {sub}: modifica un servizio di sistema",
                  f"systemctl {sub}: changes a system service")

    def p_service(self, prog, args, words):
        if args[-1:] == ["status"] or args == ["--status-all"]:
            return
        self.a.add(DANGER, "service_system", "modifica un servizio di sistema", "changes a system service")

    def p_mount(self, prog, args, words):
        if not args or args in (["-l"], ["--list"]) or (prog == "swapon" and args in (["--show"], ["-s"])):
            return
        self.a.add(DANGER, "mount", "monta o smonta dischi (richiede root)", "mounts or unmounts disks (needs root)")

    p_umount = p_mount
    p_swapoff = p_mount
    p_swapon = p_mount

    def p_sysctl(self, prog, args, words):
        if any(x in ("-w", "--write", "-p", "--load", "--system") or "=" in x for x in args):
            self.a.add(DANGER, "sysctl", "cambia parametri del kernel", "changes kernel parameters")

    def p_modprobe(self, prog, args, words):
        if any(x in ("-n", "--dry-run", "-c", "--showconfig") for x in args):
            return
        self.a.add(DANGER, "kernel_module", "carica o rimuove moduli del kernel", "loads or removes kernel modules")

    p_rmmod = p_modprobe
    p_insmod = p_modprobe

    def p_ip(self, prog, args, words):
        if any(x in ("add", "del", "delete", "set", "flush", "change", "replace", "append") for x in args):
            self.a.add(DANGER if "flush" in args or "down" in args else CHANGE, "network",
                       "modifica la configurazione di rete", "changes network configuration")

    def p_nmcli(self, prog, args, words):
        if any(x in ("delete", "down", "modify", "add", "up", "connect", "disconnect", "off", "on",
                     "reload", "import") for x in args):
            if "networking" in args and "off" in args or "radio" in args and "off" in args:
                self.a.add(DANGER, "network_off", "spegne la rete", "turns networking off")
            self.a.add(CHANGE, "network", "modifica la connessione di rete", "changes the network connection")

    def p_timedatectl(self, prog, args, words):
        if args[:1] and args[0].startswith("set"):
            self.a.add(DANGER, "system_settings", "cambia data, ora, lingua o nome del sistema",
                       "changes system date/time, locale or hostname")

    p_localectl = p_timedatectl
    p_hostnamectl = p_timedatectl

    def p_loginctl(self, prog, args, words):
        if args[:1] and args[0] in ("terminate-session", "terminate-user", "kill-session", "kill-user",
                                     "poweroff", "reboot", "lock-sessions", "suspend", "hibernate"):
            self.a.add(DANGER, "session", "termina sessioni o cambia lo stato del sistema",
                       "terminates sessions or changes system state")

    def p_journalctl(self, prog, args, words):
        if any(x.startswith("--vacuum") or x in ("--rotate", "--flush") for x in args):
            self.a.add(CHANGE, "logs", "cancella o ruota i log di sistema", "deletes or rotates system logs")

    def p_dmesg(self, prog, args, words):
        if any(x in ("-c", "-C", "--clear", "--read-clear") for x in args):
            self.a.add(CHANGE, "logs", "svuota il registro del kernel", "clears the kernel log")

    def p_history(self, prog, args, words):
        if any(x in ("-c", "-d", "-w", "-r", "-a") for x in args):
            self.a.add(CHANGE, "history_clear", "modifica o cancella la cronologia", "changes or clears history")

    def p_export(self, prog, args, words):
        for x in args:
            name = x.split("=", 1)[0]
            if name in SENSITIVE_VARS:
                self.a.add(CHANGE, "env_sensitive", f"modifica la variabile sensibile {name}",
                           f"sets sensitive variable {name}")

    def p_alias(self, prog, args, words):
        if args:
            self.a.add(CHANGE, "alias", "definisce un alias nella shell corrente", "defines an alias in this shell")

    # ------------------------------------------------------------------ desktop / settings
    def p_xrandr(self, prog, args, words):
        if any(x in ("--output", "--mode", "--rate", "--off", "--rotate", "--scale", "--auto") for x in args):
            self.a.add(CHANGE, "display", "cambia la configurazione dello schermo", "changes display configuration")

    def p_pactl(self, prog, args, words):
        if args[:1] and args[0].startswith(("set-", "load-", "unload-", "suspend", "move-")):
            self.a.add(CHANGE, "audio", "cambia le impostazioni audio", "changes audio settings")

    p_wpctl = p_pactl

    def p_powerprofilesctl(self, prog, args, words):
        if args[:1] in (["set"], ["launch"]):
            self.a.add(CHANGE, "settings", "cambia il profilo energetico", "changes the power profile")

    def p_upower(self, prog, args, words):
        return

    def p_gsettings(self, prog, args, words):
        if args[:1] and args[0] in ("set", "reset", "reset-recursively"):
            self.a.add(CHANGE, "settings", "cambia un'impostazione del desktop", "changes a desktop setting")

    def p_kwriteconfig6(self, prog, args, words):
        self.a.add(CHANGE, "settings", "cambia un'impostazione del desktop", "changes a desktop setting")

    p_kwriteconfig5 = p_kwriteconfig6
    p_plasma_apply_colorscheme = p_kwriteconfig6
    p_plasma_apply_lookandfeel = p_kwriteconfig6
    p_plasma_apply_wallpaperimage = p_kwriteconfig6
    p_lookandfeeltool = p_kwriteconfig6

    def p_vela_ai_config(self, prog, args, words):
        if args[:1] == ["set"]:
            self.a.add(CHANGE, "settings", "cambia la configurazione dell'IA", "changes the AI configuration")

    def p_qdbus(self, prog, args, words):
        if any(re.search(r"logout|shutdown|reboot|halt|poweroff", x, re.I) for x in args):
            self.a.add(DANGER, "power", "chiude la sessione o spegne il computer", "logs out or shuts down")
        elif len([x for x in args if not x.startswith("-")]) >= 3:
            self.a.add(CHANGE, "dbus_call", "invoca un'azione su un'applicazione via D-Bus",
                       "invokes an application action over D-Bus")

    p_qdbus6 = p_qdbus

    def p_busctl(self, prog, args, words):
        if args[:1] and args[0] in ("call", "set-property", "emit"):
            self.p_qdbus(prog, args + ["a", "b", "c"], words)

    def p_dbus_send(self, prog, args, words):
        self.p_qdbus(prog, args + ["a", "b", "c"], words)

    def p_ollama(self, prog, args, words):
        sub = args[0] if args else ""
        if sub in ("list", "ls", "ps", "show", "--version", "-v", "help", ""):
            return
        if sub == "rm":
            self.a.add(CHANGE, "ollama_rm", "rimuove un modello IA scaricato", "removes a downloaded AI model")
        elif sub == "pull":
            self.a.add(CHANGE, "download", "scarica un modello IA (può pesare alcuni GB)",
                       "downloads an AI model (may be several GB)")
        else:
            self.a.add(CHANGE, "ollama", f"ollama {sub}: avvia o modifica modelli IA", f"ollama {sub}")

    def p_openssl(self, prog, args, words):
        if args[:1] and args[0] in ("version", "x509", "s_client", "list", "dgst", "help", "rand", "ciphers"):
            if "-out" not in args:
                return
        self.a.add(CHANGE, "crypto", "genera o converte chiavi o file cifrati",
                   "generates or converts keys or encrypted files")

    def p_docker(self, prog, args, words):
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub in ("ps", "images", "logs", "inspect", "version", "info", "stats", "top", ""):
            return
        if any(x in ("prune", "rm", "rmi") for x in args):
            self.a.add(DANGER, "docker_prune", "cancella container, immagini o volumi",
                       "deletes containers, images or volumes")
        if sub == "run" and any(x == "--privileged" or x.startswith(("-v/:", "/:")) or x == "/:/host" for x in args):
            self.a.add(DANGER, "docker_privileged", "container con accesso completo al sistema",
                       "container with full system access")
        self.a.add(CHANGE, "docker", f"{prog} {sub} modifica container o immagini",
                   f"{prog} {sub} changes containers or images")

    p_podman = p_docker

    # ------------------------------------------------------------------ network
    def p_curl(self, prog, args, words):
        a = self.a
        if any(x in ("-T", "--upload-file", "-d", "--data", "--data-binary", "--data-raw", "-F", "--form",
                     "--data-urlencode", "--json") or x.startswith(("--data", "-d@", "-F")) for x in args):
            a.add(CHANGE, "network_send", "invia dati o file in rete", "sends data or files over the network")
        if any(x in ("-o", "-O", "--output", "--remote-name", "-J", "--remote-name-all") or
               (x.startswith("-o") and len(x) > 2) or (x.startswith("-") and not x.startswith("--") and "O" in x)
               for x in args):
            a.add(CHANGE, "download", "scarica un file", "downloads a file")
        if any(x in ("-X", "--request") for x in args) and any(x.upper() in ("DELETE", "PUT", "POST", "PATCH") for x in args):
            a.add(CHANGE, "network_send", "invia una richiesta che modifica dati remoti",
                  "sends a request that changes remote data")

    def p_wget(self, prog, args, words):
        if "--post-file" in args or "--post-data" in args:
            self.a.add(CHANGE, "network_send", "invia dati in rete", "sends data over the network")
        if any(x in ("-O-", "-qO-", "--output-document=-") for x in args) or (
                "-O" in args and args[args.index("-O") + 1:args.index("-O") + 2] == ["-"]):
            return
        if "--spider" in args:
            return
        self.a.add(CHANGE, "download", "scarica file", "downloads files")

    def p_scp(self, prog, args, words):
        self.a.add(CHANGE, "network_send", "copia file da o verso un altro computer",
                   "copies files to or from another computer")

    p_sftp = p_scp

    def p_ssh(self, prog, args, words):
        self.a.add(CHANGE, "remote", "si collega a un altro computer", "connects to another computer")

    def p_nc(self, prog, args, words):
        self.a.add(CHANGE, "network_raw", "apre connessioni di rete grezze", "opens raw network connections")

    p_ncat = p_nc
    p_netcat = p_nc
    p_socat = p_nc

    # ------------------------------------------------------------------ redirections
    def write_target(self, target: str, append=False):
        a = self.a
        t = target
        if t in SAFE_DEVICES or t in ("1", "2", "-") or t.startswith("&"):
            return
        if is_block_device(t):
            a.add(DANGER, "write_device", f"scrive direttamente sul dispositivo {t}: può distruggere un disco",
                  f"writes directly to device {t}: can destroy a disk")
        elif is_system_path(t):
            a.add(DANGER, "write_system", f"scrive su un file di sistema ({t})", f"writes to a system file ({t})")
        elif touches_rc(t):
            a.add(CHANGE, "write_rc", f"modifica una configurazione importante ({t})",
                  f"changes important configuration ({t})")
        if append:
            a.add(CHANGE, "append", f"aggiunge testo al file {t}", f"appends to file {t}")
        else:
            a.add(CHANGE, "overwrite", f"crea o sovrascrive il file {t}", f"creates or overwrites file {t}")

    def redirects(self, cmd: SimpleCommand):
        for r in cmd.redirects:
            if r.op in (">", ">|", ">>", "<>") or (r.op == ">&" and not r.target.isdigit() and r.target != "-"):
                self.write_target(r.target, append=(r.op == ">>"))


FORK_BOMB_RE = re.compile(r"([\w:.]+)\s*\(\s*\)\s*\{[^}]*\1\s*\|\s*\1\s*&[^}]*\}\s*;?\s*\1")
WRAPPER_NAMES = PRIV | {"env", "nice", "nohup", "time", "command", "exec", "stdbuf", "timeout",
                        "ionice", "builtin", "setsid", "unbuffer"}


def _effective_program(cmd: SimpleCommand) -> str:
    words = [w.value for w in cmd.words]
    while words and ASSIGN_RE.match(words[0]):
        words = words[1:]
    while words:
        p = _base(words[0])
        if p in WRAPPER_NAMES:
            words = words[1:]
            first = True
            while words and (words[0].startswith("-") or "=" in words[0] or
                             (p == "timeout" and first and re.match(r"^\d", words[0]))):
                if words[0] in ("-u", "-g", "-s", "-k", "-n", "-c") and len(words) > 1:
                    words = words[1:]
                words = words[1:]
                first = False
            if p == "timeout" and words and re.match(r"^\d", words[0]):
                words = words[1:]
            continue
        return p
    return ""


def _effective_args(cmd: SimpleCommand):
    vals = [w.value for w in cmd.words]
    prog = _effective_program(cmd)
    if prog in vals:
        return vals[vals.index(prog) + 1:]
    for i, v in enumerate(vals):
        if _base(v) == prog:
            return vals[i + 1:]
    return []


def _reads_stdin_as_code(cmd: SimpleCommand) -> bool:
    """True if an interpreter in a pipeline would execute what it reads on stdin."""
    args = _effective_args(cmd)
    if any(a in ("-c", "-e", "--eval", "-E") for a in args):
        return False
    pos = [a for a in args if not a.startswith("-")]
    if not pos:
        return True
    if pos[0] in ("-", "/dev/stdin"):
        return True
    return False


def _programs_in(text: str):
    out = []
    try:
        res = parse(text)
    except Exception:
        return out
    for p in res.pipelines:
        for c in p.commands:
            out.append(_effective_program(c))
    return out


def _analyze(text: str, a: Assessment, depth=0):
    if FORK_BOMB_RE.search(text):
        a.add(DANGER, "fork_bomb", "fork bomb: blocca il computer creando processi all'infinito",
              "fork bomb: freezes the machine by spawning processes endlessly")
    res = parse(text)
    an = _Analyzer(a, depth)
    if res.uncertain:
        a.uncertain = True
        a.add(CHANGE, "parse", "comando non analizzabile con certezza (virgolette o parentesi non chiuse?)",
              "command cannot be analysed reliably (unclosed quotes or brackets?)")
    sub_downloads = any(set(_programs_in(s)) & DOWNLOADERS for s in res.substitutions)
    for pipe in res.pipelines:
        prev = []
        for idx, cmd in enumerate(pipe.commands):
            eff = _effective_program(cmd)
            if idx > 0 and (eff in INTERPRETERS or re.fullmatch(r"python[23]?(\.\d+)?", eff or "-")) \
                    and _reads_stdin_as_code(cmd):
                if set(prev) & DOWNLOADERS:
                    a.add(DANGER, "curl_pipe_shell", f"scarica codice da Internet e lo esegue subito con {eff}",
                          f"downloads code from the Internet and runs it immediately with {eff}")
                else:
                    a.add(DANGER, "pipe_shell", f"esegue con {eff} codice prodotto da un altro comando",
                          f"runs code produced by another command with {eff}")
            if idx > 0 and eff in ("xargs", "parallel") and set(prev) & DOWNLOADERS:
                a.add(DANGER, "download_xargs", "usa dati scaricati da Internet come comandi",
                      "uses data downloaded from the Internet as commands")
            if sub_downloads and (eff in INTERPRETERS or eff in ("eval", "source", ".")):
                a.add(DANGER, "curl_exec", "scarica codice da Internet e lo esegue",
                      "downloads code from the Internet and runs it")
            an.command(cmd)
            an.redirects(cmd)
            prev.append(eff)
    for sub in res.substitutions:
        an.nested(sub)
    for cmd, body in res.heredocs:
        eff = _effective_program(cmd)
        if eff in SHELLS and _reads_stdin_as_code(cmd):
            an.nested(body)
        elif eff in INTERPRETERS and _reads_stdin_as_code(cmd):
            a.add(CHANGE, "heredoc_code", f"esegue codice {eff} incluso nel comando", f"runs embedded {eff} code")
            if re.search(r"rmtree|os\.remove|unlink|system\(|subprocess|rm -rf|shutil\.", body):
                a.add(DANGER, "inline_code_destructive", "codice incluso che cancella file o lancia comandi",
                      "embedded code that deletes files or spawns commands")


def classify(command: str) -> Assessment:
    """Classify a shell command line. Never raises."""
    a = Assessment(command=command or "")
    text = (command or "").strip()
    if not text:
        return a
    try:
        _analyze(text, a)
    except Exception as e:  # defensive: never let a bug downgrade risk
        a.uncertain = True
        a.add(DANGER, "classifier_error", "errore nell'analisi del comando: trattato come pericoloso",
              f"classifier error ({e}): treated as dangerous")
    low = text.lower()
    have = {r.code for r in a.reasons}
    # raw-text belt and braces (catch what the parser might miss)
    if re.search(r"\b(curl|wget|fetch)\b[^|;&]*\|\s*(sudo\s+(-\S+\s+)*)?(env\s+)?(ba|z|da|k|fi|c|tc)?sh\b", low) \
            and "curl_pipe_shell" not in have:
        a.add(DANGER, "curl_pipe_shell", "scarica codice da Internet e lo esegue subito",
              "downloads code from the Internet and runs it immediately")
    if re.search(r">\s*/dev/(sd[a-z]|nvme\d|vd[a-z]|hd[a-z]|mmcblk\d|xvd[a-z])", low) \
            and "write_device" not in have:
        a.add(DANGER, "write_device", "scrive direttamente su un disco", "writes directly to a disk")
    if re.search(r"(^|[\s;&|(])(sudo|doas|pkexec|run0)(\s|$)", low) and a.level != DANGER:
        a.add(DANGER, "privilege", "usa privilegi di amministratore (root)", "uses administrator (root) privileges")
    if re.search(r"(^|[\s;&|(])rm\s+(-\S*[rR]\S*\s+)*-\S*f|rm\s+-\S*f\S*\s+-\S*[rR]|rm\s+-\S*[rR]\S*f", low) \
            and a.level != DANGER:
        a.add(DANGER, "rm_rf", "rm -rf cancella cartelle intere senza chiedere e senza cestino",
              "rm -rf deletes whole directories without asking and without trash")
    if a.uncertain and a.level == SAFE:
        a.add(CHANGE, "uncertain", "analisi incerta", "uncertain analysis")
    if a.level == SAFE and not a.reasons:
        a.reasons.append(Reason(SAFE, "readonly", "solo lettura: non modifica nulla", "read-only: changes nothing"))
    return a


def level_label(level: str, lang="it") -> str:
    it = {SAFE: "Sicuro", CHANGE: "Modifica", DANGER: "Pericoloso"}
    en = {SAFE: "Safe", CHANGE: "Changes things", DANGER: "Dangerous"}
    return (it if lang == "it" else en).get(level, level)


if __name__ == "__main__":  # python3 -m vela_terminal.danger 'cmd' ...
    import json
    import sys
    for c in sys.argv[1:]:
        print(json.dumps(classify(c).to_dict(), ensure_ascii=False, indent=1))
