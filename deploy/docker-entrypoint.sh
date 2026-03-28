#!/bin/sh
set -eu

runtime_config="/app/config/runtime.toml"

mkdir -p /app/config /data

cat >"$runtime_config" <<EOF
[http]
bind = "${SDNS_HTTP_BIND:-0.0.0.0:8088}"

[store]
database_url = "${SDNS_STORE_DATABASE_URL:-postgres://semantic_dns:semantic_dns@postgres:5432/semantic_dns}"
schema = "${SDNS_STORE_SCHEMA:-semantic_dns}"

[dns]
zone = "${SDNS_DNS_ZONE:-local}"
zone_file = "${SDNS_ZONE_FILE:-/data/semantic-dns.zone}"

[audit]
database_url = "${SDNS_AUDIT_DATABASE_URL:-postgres://semantic_dns:semantic_dns@postgres:5432/semantic_dns}"
schema = "${SDNS_AUDIT_SCHEMA:-semantic_dns}"

[fathom]
database_url = "${SDNS_FATHOM_DATABASE_URL:-}"

[[api_tokens]]
name = "local-admin"
token = "${SDNS_ADMIN_TOKEN:-semantic-admin-token}"
role = "admin"

[[api_tokens]]
name = "dhcp-engine"
token = "${SDNS_DHCP_TOKEN:-semantic-dhcp-token}"
role = "system"
EOF

exec /usr/local/bin/semantic-dns --config "$runtime_config"
