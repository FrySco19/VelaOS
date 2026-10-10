#!/bin/sh
set -eu
chmod 755 /usr/lib/vela/local-ai/llama-server
test -x /usr/lib/vela/local-ai/llama-server
test -s /usr/share/vela/models/gemma-4-E2B.gguf
sha256sum -c /usr/share/vela/models/SHA256SUMS
mkdir -p /etc/systemd/user/default.target.wants
ln -sf /usr/lib/systemd/user/vela-local-ai.service /etc/systemd/user/default.target.wants/vela-local-ai.service
