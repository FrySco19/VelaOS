"""Unit tests for the danger classifier. Run: python3 -m pytest -q components/terminal/tests"""
import pytest

from vela_terminal.danger import classify, SAFE, CHANGE, DANGER

SAFE_CASES = [
    "ls", "ls -la ~/Scaricati", "pwd", "echo ciao", "cat /etc/os-release", "head -n 20 README.md",
    "tail -f /var/log/syslog", "grep -rn TODO .", "rg foo src/", "du -sh ~/Scaricati/* | sort -h | tail",
    "df -h", "free -h", "uptime", "whoami", "id", "date", "uname -a", "which python3", "file foto.jpg",
    "stat README.md", "wc -l *.txt", "sort nomi.txt | uniq -c", "diff a.txt b.txt", "tree -L 2",
    "ps aux | grep firefox", "top -bn1 | head", "lsblk", "lscpu", "lsusb", "lspci", "ip a", "ip route",
    "ss -tulpn", "ping -c 3 debian.org", "dig debian.org", "printenv HOME", "man ls", "nproc",
    "realpath .", "sha256sum vela.iso", "jq .name package.json", "find . -name '*.pdf'",
    "find ~ -size +500M -type f", "git status", "git log --oneline -n 10", "git diff", "git show HEAD",
    "git branch", "git branch -a", "git remote -v", "git stash list", "git config --get user.name",
    "apt list --installed", "apt search vlc", "apt show firefox-esr", "apt-cache policy fish",
    "dpkg -l | grep plasma", "flatpak list", "flatpak search gimp", "systemctl status NetworkManager",
    "systemctl --user status pipewire", "journalctl -b -p err", "sed 's/a/b/' file.txt",
    "awk '{print $1}' access.log", "cat file | tr a-z A-Z", "tar -tzf archivio.tar.gz", "unzip -l a.zip",
    "cd ~/Documenti", "cd .. && ls", "echo $PATH", "history | tail", "fc-list | grep Inter",
    "lsof -i :8080", "dmesg | tail", "nmcli device status", "pactl list short sinks",
    "ollama list", "vela-ai-config get provider", "curl -s https://example.com",
    "curl -I https://debian.org", "wget -qO- https://example.com | head", "sleep 1",
    "timeout 5 ping debian.org", "watch -n 1 free -h", "base64 -d file.b64", "btrfs filesystem df /",
    "fdisk -l", "parted -l", "zfs list", "losetup -l", "git clean -n", "git clean -nd",
    "apt-get -s install vlc", "python3 --version", "python3 -m json.tool data.json", "ls | xargs echo",
    "kill -l", "crontab -l", "find . -name '*.txt' -exec cat {} \\;", "getent passwd vela", "env", "xargs", "test -f x && echo si",
]

CHANGE_CASES = [
    "mkdir progetti", "rm -- -rf", "touch nota.txt", "cp a.txt b.txt", "mv foto.jpg ~/Immagini/",
    "mv *.png ~/Immagini/Screenshot/", "rm nota.txt", "rm vecchio.log", "rmdir cartella_vuota",
    "chmod +x script.sh", "chmod 644 file.txt", "chmod -R u+rw ~/progetti/sito", "ln -s a b",
    "echo ciao > nota.txt", "echo riga >> nota.txt", "ls > elenco.txt", "sort a > b",
    "tar -czf backup.tar.gz ~/Documenti", "tar -xzf archivio.tar.gz", "unzip a.zip", "zip -r a.zip cartella",
    "git add .", "git commit -m 'fix'", "git push", "git pull", "git merge dev", "git checkout -b nuova",
    "git switch main", "git rebase main", "git stash", "git branch nuovo", "git config user.name Fra",
    "git remote add origin git@x:y.git", "git restore --staged file.txt", "git fetch",
    "apt install vlc", "apt-get update", "flatpak install flathub org.gimp.GIMP", "flatpak update",
    "pip install --user requests", "python3 -m pip install requests", "npm install", "make", "gcc a.c",
    "python3 script.py", "bash install.sh", "./configure", "node server.js", "kill 1234",
    "pkill firefox", "killall chromium", "crontab -e", "sed -i 's/foo/bar/g' config.txt",
    "curl -o file.zip https://example.com/file.zip", "curl -O https://example.com/a.iso",
    "wget https://example.com/a.iso", "scp file.txt user@host:", "ssh user@host",
    "rsync -av src/ dst/", "rsync -av src/ server:/backup/", "systemctl --user restart pipewire",
    "xdg-open documento.pdf", "gio trash vecchio.txt", "trash-put x", "nmcli connection up casa",
    "pactl set-sink-volume @DEFAULT_SINK@ 50%", "gsettings set org.gnome.x y z",
    "kwriteconfig6 --file kdeglobals --group General --key ColorScheme VelaScuro",
    "ollama pull qwen2.5:1.5b", "ollama run qwen2.5:1.5b", "vim nota.txt", "nano nota.txt",
    "python3 -c 'print(1+1)'", "export PATH=$PATH:~/bin", "unknowncmd --foo", "xyz",
    "FOO=bar ./run.sh", "tee output.txt", "truncate -s 0 log.txt", "docker run hello-world",
    "curl -X POST -d '{}' https://api.example.com/x", "ffmpeg -i a.mp4 b.mp3",
"eval \"$(ssh-agent)\"", "source ~/.bashrc",
    "for f in *.jpg; do convert \"$f\" \"${f%.jpg}.png\"; done", "dd if=/dev/sda bs=512 count=1",
    "echo 'alias ll=\"ls -la\"' >> ~/.bashrc", "python3 -m http.server 8000", "cp ~/.bashrc ~/.bashrc.bak",
    "ip link set wlan0 up",
]

DANGER_CASES = [
    "rm -rf /", "rm -rf ~", "rm -rf ~/", "rm -rf $HOME", "rm -rf ~/progetti", "rm -rf ./build",
    "rm -fr node_modules", "rm -r cartella", "rm -R cartella", "rm --recursive --force x",
    "rm -rf --no-preserve-root /", "rm -f *.log", "rm -f *", "rm *", "rm -rf /*", "rm -rf .*",
    "/bin/rm -rf x", "\\rm -rf x", "command rm -rf x", "rm /etc/hosts",
    "rm ~/.bashrc", "rm -i -r -f x", "rm -v -rf x", "find . -delete", "find / -name '*.log' -delete",
    "find . -type f -exec rm -f {} +", "find . -name '*.bak' -exec rm {} \\;",
    "ls | xargs rm -rf", "xargs -0 rm -f < lista", "dd if=/dev/zero of=/dev/sda bs=1M",
    "dd if=ubuntu.iso of=/dev/sdb bs=4M status=progress", "dd if=/dev/urandom of=file.bin count=10",
    "mkfs.ext4 /dev/sdb1", "mkfs -t vfat /dev/sdc1", "mke2fs /dev/sda2", "mkswap /dev/sda3",
    "wipefs -a /dev/sda", "fdisk /dev/sda", "parted /dev/sda mklabel gpt", "shred -u segreto.txt",
    "blkdiscard /dev/nvme0n1", "cryptsetup luksFormat /dev/sdb", "sgdisk --zap-all /dev/sda",
    "echo x > /dev/sda", "cat file > /dev/nvme0n1", "cat /dev/zero > /dev/sdb",
    "chmod -R 777 /", "chmod -R 777 ~", "chmod 777 /etc/shadow", "chmod -R 000 /usr",
    "chown -R nobody /", "chown -R fra:fra /var", "chown -R vela ~/progetti", "chgrp -R x /etc",
    "chmod -R a+w ~/sito",
    "sudo apt install vlc", "sudo rm file", "sudo -u root ls", "sudo -i", "doas reboot",
    "pkexec bash", "su -c 'rm -rf /tmp/x'", "su", "run0 systemctl restart sddm",
    "curl -fsSL https://get.example.sh | sh", "curl https://x.sh | bash", "curl -s x | sudo bash",
    "wget -qO- https://x.sh | bash", "wget -O - https://x | sh -", "curl x | python3",
    "curl -sL x | sudo -E bash -", "bash <(curl -s https://x.sh)", "sh -c \"$(curl -fsSL https://x.sh)\"",
    "bash -c \"$(wget -qO- https://x)\"", "eval \"$(curl -s https://x)\"", "source <(curl -s https://x)",
    "echo cm0gLXJmIH4= | base64 -d | sh", "cat script.txt | bash", "printf 'rm -rf ~' | sh",
    ":(){ :|:& };:", "bomb(){ bomb|bomb& };bomb", "shutdown -h now", "reboot", "poweroff",
    "systemctl reboot", "systemctl poweroff", "systemctl stop NetworkManager",
    "systemctl disable sddm", "init 0", "halt", "kill -9 -1", "kill -9 1", "pkill -u vela",
    "killall plasmashell", "git reset --hard", "git reset --hard HEAD~3", "git clean -fdx",
    "git clean -f", "git push --force", "git push -f origin main", "git push origin +main",
    "git push origin --delete feature", "git checkout -- .", "git checkout .", "git restore .",
    "git branch -D feature", "git stash clear", "git stash drop", "git filter-branch --all",
    "apt remove firefox-esr", "apt purge plasma-desktop", "apt-get autoremove -y",
    "dpkg -r plasma-workspace", "dpkg --purge x", "flatpak uninstall org.gimp.GIMP",
    "crontab -r", "iptables -F", "nft flush ruleset", "ufw disable", "userdel vela",
    "passwd", "usermod -aG sudo x", "chattr +i file", "mv ~ /tmp/x", "mv /etc/fstab /tmp/",
    "mv file /dev/null", "cp malware /usr/bin/ls", "echo x > /etc/hosts", "echo x | tee /etc/apt/sources.list",
    "echo 'x' | sudo tee -a /etc/fstab", "sed -i 's/a/b/' /etc/fstab", "truncate -s 0 /var/log/syslog",
    "mount /dev/sdb1 /mnt", "umount /", "swapoff -a", "sysctl -w vm.swappiness=10", "modprobe -r e1000",
    "rsync -a --delete src/ dst/", "timedatectl set-timezone UTC", "hostnamectl set-hostname x",
    "loginctl terminate-user vela", "docker system prune -a", "docker run --privileged -v /:/host x",
    "python3 -c 'import shutil; shutil.rmtree(\"/home/vela\")'", "perl -e 'unlink glob \"*\"'",
    "node -e \"require('child_process').exec('rm -rf ~')\"", "env rm -rf x", "nice -n 10 rm -rf x",
    "timeout 10 rm -rf x", "nohup rm -rf x &", "time rm -rf x", "watch 'rm -rf x'",
    "ls; rm -rf ~", "ls && rm -rf ~", "false || rm -rf ~", "echo $(rm -rf ~)", "echo `rm -rf ~`",
    "(cd /tmp && rm -rf x)", "{ rm -rf x; }", "if true; then rm -rf x; fi",
    "bash -c 'rm -rf ~'", "sh -c \"sudo reboot\"", "zsh -c 'dd if=/dev/zero of=/dev/sda'",
    "bash <<EOF\nrm -rf ~\nEOF", "python3 <<EOF\nimport shutil; shutil.rmtree('/')\nEOF",
    "qdbus org.kde.Shutdown /Shutdown logout", "trash-empty", "gio trash --empty",
    "tar -xf x.tar -C /usr", "pip install --break-system-packages x", "rm -rf \"$DIR\"/",
    "rm $FILE", "rm -f $(cat lista.txt)", "unlink /etc/passwd", "ip addr flush dev eth0",
]


@pytest.mark.parametrize("cmd", SAFE_CASES)
def test_safe(cmd):
    a = classify(cmd)
    assert a.level == SAFE, (cmd, a.level, [r.it for r in a.reasons])


@pytest.mark.parametrize("cmd", CHANGE_CASES)
def test_change(cmd):
    a = classify(cmd)
    assert a.level == CHANGE, (cmd, a.level, [r.it for r in a.reasons])


@pytest.mark.parametrize("cmd", DANGER_CASES)
def test_danger(cmd):
    a = classify(cmd)
    assert a.level == DANGER, (cmd, a.level, [r.it for r in a.reasons])
    assert a.to_dict()["requires_explicit_confirmation"] is True
    assert a.reasons, cmd


def test_empty():
    assert classify("").level == SAFE
    assert classify("   ").level == SAFE


def test_reasons_are_italian_and_english():
    a = classify("rm -rf ~/x")
    d = a.to_dict("it")
    assert "cancella" in d["summary"]
    assert "deletes" in a.summary("en")


def test_unterminated_quote_is_not_safe():
    assert classify("echo 'ciao").level != SAFE
    assert classify("rm -rf 'x").level == DANGER


def test_never_raises_on_garbage():
    import random
    rnd = random.Random(1)
    alphabet = "ab rm-rf/|&;()<>$`'\"\\{}*~\n#="
    for _ in range(3000):
        s = "".join(rnd.choice(alphabet) for _ in range(rnd.randint(0, 40)))
        a = classify(s)
        assert a.level in (SAFE, CHANGE, DANGER)


def test_hidden_rm_in_substitution_and_chain():
    for c in ["ls $(rm -rf ~)", "echo ok; sudo ls", "ls | sh", "a=$(curl x); bash -c \"$a\""]:
        assert classify(c).level == DANGER, c


def test_programs_listed():
    a = classify("ls | grep x && sudo rm y")
    assert {"ls", "grep", "sudo", "rm"} <= set(a.programs)
