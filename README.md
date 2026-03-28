# Semantic DNS

Semantic DNS is an ISA-95-aligned, UNS-adjacent network context source for
environments that need names to reflect what a device is and where it belongs
in the process, not just whatever hostname it happened to ask for. It turns
asset observations into stable, meaningful DNS records and serves that semantic
view as a real DNS zone.

![Semantic DNS operator console](screenshots/hero.png)

The naming model is now explicitly split into two layers:

- DNS FQDNs carry network and ISA-95 placement context in resolvable,
  leaf-first form such as `CaseRobot.Palletizer.Packout.Zone4.Milwaukee.local`
- hardware identities such as MAC addresses, serial numbers, or future x509
  certificate fingerprints bind the record to a real physical or cryptographic
  endpoint
- application identities such as UNIs and URNs are carried alongside the DNS
  record as stable identifiers that do not have to change when a device moves
  in the hierarchy

Instead of treating DNS as a separate manual inventory problem, Semantic DNS
lets you ingest DHCP and discovery data, merge that data into a semantic record
for each device, publish a zone file, and keep an audit trail of how those
names were assigned. In the default deployment, the semantic service maintains
the zone data and the bundled CoreDNS layer serves it on TCP/UDP port `53`.

That means you can move from raw sightings like MAC addresses, DHCP
fingerprints, or imported asset inventory to names such as
`DriveVFD.Conveyor.Cell5.Zone3.Milwaukee.local` with attached metadata and
provenance. The canonical DNS order is leaf-first so each parent label can be a
meaningful resolvable network context: `leaf.work-unit.work-center.area.site.local`.

## What It Enables

- Serves a DNS zone whose records are derived from operational asset context
  instead of flat, manually curated host lists.
- Builds stable DNS names from ISA-95-aligned context such as site, area, work
  center, work unit, and function instead of relying on ad hoc hostnames
  alone.
- Merges multiple observation sources into one semantic record, including
  manual API input, protocol analysis, switch intelligence, DHCP
  fingerprinting, discovery, and replacement inference.
- Preserves field-level provenance so you can see where values such as class,
  vendor, model, or status came from and how confident the record is.
- Resolves devices by FQDN, internal IP, or external IP through the API.
- Publishes a DNS zone file and companion TXT records that expose semantic
  metadata alongside A records.
- Tracks DHCP leases, quarantine queues, fingerprint rules, role templates, and
  reconciliation status for the DNS publishing workflow.
- Emits signed audit events for security- and operations-relevant actions such
  as observation ingest and quarantine authorization.
- Streams record updates over WebSockets so other tooling can react to changes
  without polling.
- Imports existing asset context from a Fathom-style PostgreSQL database so the
  DNS view can be bootstrapped from an inventory source of truth.
- Provides a network context layer that can publish into a UNS or adjacent
  systems without requiring Semantic DNS itself to be the broker of record.

## Identity Model

Semantic DNS now models three related but different identity surfaces:

- `fqdn`: the canonical resolvable DNS name for the current network and ISA-95
  placement
- `hardware_identities`: hardware or certificate-linked identities such as MAC
  addresses, serial numbers, DHCP client IDs, or x509-derived values
- `application_identities`: stable application-level identifiers such as
  `urn:mes:asset:case-robot-204` or `uni://packout/palletizer/caserobot`
- `aliases`: human-friendly or legacy lookup names that should still resolve to
  the current canonical record

That means:

- DNS is for resolution and topology context
- hardware identities are for binding a record to a real device or trust anchor
- UNI and URN identifiers are for application identity and cross-system joins
- aliases are compatibility handles for operators and older tooling

Records can also carry `relations` like `located-in`, `served-by`, or
`reports-to` so applications can reason about how an asset sits in a broader
context graph without encoding those relationships in the hostname.

Application identities are searchable and resolvable through the API store
layer, but they do not become additional DNS labels.

## What You Deploy

The default deployment is a small DNS stack:

- `semantic-dns`: the semantic control plane that accepts observations,
  maintains records, writes the zone file, and exposes the HTTP and WebSocket
  API on port `8088`
- `postgres`: persistent storage for records, leases, templates, fingerprints,
  audit history, and sync state
- `dns` via CoreDNS: the DNS serving layer that answers queries on TCP/UDP
  port `53` using the zone file maintained by Semantic DNS

In other words, Semantic DNS is not just a library or data model. The shipped
deployment runs an actual DNS server in front of a semantic naming engine.

## How It Works

1. A system or operator submits an observation with whatever facts are known:
   IPs, MAC, vendor, model, protocols, ISA-95 hierarchy context, and status.
2. Semantic DNS merges that observation into a canonical semantic record and
   derives a stable FQDN from the available operational context.
3. The DNS publisher rewrites the zone file so the current semantic view is
   available to downstream DNS infrastructure.
4. The service records an audit event and broadcasts the updated record to
   subscribers.

For manual record creation, Semantic DNS now treats ISA-95 hierarchy nodes as
first-class records. That means a device like
`CaseRobot.Palletizer.Packout.Zone4.Milwaukee.local` is expected to sit beneath
existing `work-unit`, `work-center`, `area`, and `site` records such as:

- `Palletizer.Packout.Zone4.Milwaukee.local`
- `Packout.Zone4.Milwaukee.local`
- `Zone4.Milwaukee.local`
- `Milwaukee.local`

Manual API writes are validated against that parent chain so devices only get
published into a hierarchy that can actually resolve step-by-step.

## Manual API Expectations

Manual API writes now enforce two things:

1. The ISA-95 hierarchy must be complete for the declared `node_kind`
2. A MAC address or another hardware identity must be present
3. Parent hierarchy records must already exist for manual `device`,
   `work-unit`, `work-center`, and `area` records

So if you want to create a device like
`CaseRobot.Palletizer.Packout.Zone4.Milwaukee.local`, the following hierarchy
records are expected to exist already:

- `Milwaukee.local` as `site`
- `Zone4.Milwaukee.local` as `area`
- `Packout.Zone4.Milwaukee.local` as `work-center`
- `Palletizer.Packout.Zone4.Milwaukee.local` as `work-unit`

Passive discovery and imports are still allowed to land partial context, but
manual authoring now reflects the stricter “every step must resolve” rule.

## Operator Console Authoring

The web console now exposes a guided hierarchy builder so operators do not need
to hand-author JSON observations.

The authoring workflow is:

1. create or select a `site`
2. create an `area` beneath that site
3. create a `work-center` beneath that area
4. create a `work-unit` beneath that work center
5. create one or more `device` records beneath that work unit

The builder shows:

- the selected `node_kind`
- a live FQDN preview such as
  `CaseRobot.Palletizer.Packout.Zone4.Milwaukee.local`
- whether each parent level already exists and resolves
- whether a MAC address or another hardware identity is present
- whether the target FQDN already exists
- optional UNI, URN, and alias inputs

This means an operator can build the hierarchy in the same order DNS resolves
it, while still attaching hardware and application identity at create time.

## DHCP And Hardware Tracking

Semantic DNS now treats hardware identity as a first-class part of the record.

- `mac` remains as a convenience field
- `hardware_identities` is the canonical extensible list for device anchoring
- DHCP lease data can enrich matching records with MAC-derived hardware
  identity when the lease can be matched by DNS name or IP
- the model is ready for future x509-linked identities like subject, SAN URI,
  or SPKI fingerprint

## Deploy The DNS Server

If you want to run Semantic DNS as a real DNS server, start here.

Prerequisites:

- Docker with Compose support
- A host where you can bind TCP/UDP port `53`

Create the runtime environment file:

```bash
cp deploy/server.env.example deploy/server.env
```

Edit `deploy/server.env` and replace the placeholder passwords and tokens.

Start the full stack:

```bash
docker compose --env-file deploy/server.env -f deploy/server-compose.yaml up -d --build
```

Once the stack is up:

- DNS answers on TCP/UDP `53`
- The HTTP API is available on `8088`
- PostgreSQL stores the semantic state and audit history
- CoreDNS serves the generated zone file from `/data/semantic-dns.zone`

The bundled CoreDNS configuration forwards non-`local` lookups to `8.8.8.8`
and serves the semantic zone described in [deploy/Corefile](deploy/Corefile).

## Local Demo

Prerequisites:

- Rust `1.88`
- Docker or another reachable PostgreSQL instance

Start the local PostgreSQL dependency:

```bash
docker compose up -d postgres
```

Run the service with the development config:

```bash
cargo run -p semantic-dns -- --config config/dev.toml
```

Start the operator console in a second terminal:

```bash
cd apps/web
npm install
npm run dev
```

Check the health endpoint with the development admin token:

```bash
curl -H 'Authorization: Bearer semantic-admin-token' \
  http://127.0.0.1:8088/api/v1/health
```

Push a sample DHCP observation:

```bash
curl -X POST \
  -H 'Authorization: Bearer semantic-dhcp-token' \
  -H 'Content-Type: application/json' \
  http://127.0.0.1:8088/api/v1/observations \
  -d '{
    "id":"11111111-1111-7111-8111-111111111111",
    "device_id":"22222222-2222-7222-8222-222222222222",
    "observed_at":"2026-03-24T18:00:00Z",
    "source":"dhcp-fingerprint",
    "external_ip":"10.50.3.47",
    "internal_ip":"192.168.1.47",
    "class":"vfd",
    "vendor":"rockwell",
    "model":"PowerFlex500",
    "protocols":["ethernet-ip"],
    "mac":"00:00:BC:3A:47:12",
    "switch_port":"Gi1/0/5",
    "enterprise":"Butterbones",
    "site":"Milwaukee",
    "area":"Zone3",
    "work_center":"Cell5",
    "work_center_kind":"process-cell",
    "work_unit":"Conveyor",
    "facility":"Milwaukee",
    "zone":"Zone3",
    "cell":"Cell5",
    "process":"Conveyor",
    "function":"DriveVFD",
    "status":"active"
  }'
```

The generated zone file is written to `./semantic-dns.zone` when using the
development config.

After ingesting the observation, you can:

- Resolve the resulting record by IP or FQDN
- Query the semantic record index
- Open the operator console at `http://127.0.0.1:5173`
- Confirm that `semantic-dns.zone` was rewritten with the new record
- Inspect DNS synchronization state and other operational APIs

Resolve the resulting record by IP or FQDN:

```bash
curl -H 'Authorization: Bearer semantic-admin-token' \
  http://127.0.0.1:8088/api/v1/resolve/192.168.1.47
```

Query the semantic record index:

```bash
curl -H 'Authorization: Bearer semantic-admin-token' \
  'http://127.0.0.1:8088/api/v1/dns/query?site=Milwaukee&work_center=Cell5&class=vfd'
```

After ingest, the service can also:

- Regenerate `semantic-dns.zone` with the current A and TXT records.
- Show DNS synchronization health at `/api/v1/dhcp/dns/sync-status`.
- Evaluate fingerprint input against stored rules and role templates.
- Import Fathom asset data with `/api/v1/integrations/fathom/import`.
- Push change notifications to WebSocket clients on `/api/v1/ws`.

## Operational Surface

The HTTP API is the control plane for the DNS server. It lets you:

- Ingest observations with `/api/v1/observations`
- Resolve names or addresses with `/api/v1/resolve/{target}`
- Query semantic records with `/api/v1/dns/query`
- Manage DHCP-related state with the `/api/v1/dhcp/*` endpoints
- Import external inventory with `/api/v1/integrations/fathom/import`
- Subscribe to change events with `/api/v1/ws`

The DNS plane is the served zone itself. Semantic DNS updates the zone file,
and CoreDNS answers queries from that generated data.

The frontend operator console lives in `apps/web`. It currently provides:

- `Records`: searchable semantic record explorer with field provenance
- `Graph`: ISA-95 hierarchy graph for sites, areas, work centers, work units,
  and attached devices
- `Operations`: synchronization, quarantine, fingerprints, and role templates
- `Audit`: recent audit ledger events exposed by the HTTP API

## ISA-95 Alignment

Semantic DNS stores ISA-95 hierarchy hints directly on observations and
semantic records, including `enterprise`, `site`, `area`, `work_center`,
`work_center_kind`, and `work_unit`.

For compatibility with older clients and existing data, it also preserves the
legacy aliases `facility`, `zone`, `cell`, and `process`. Those aliases map to
`site`, `area`, `work_center`, and `work_unit` respectively.

## Architecture Notes

Implementation note: the codebase is a Rust workspace, but the product it
builds is a DNS server and its supporting control plane.

- `semantic-dns`: binary that wires the system together
- `sdns-api`: Axum HTTP API and WebSocket event stream
- `sdns-bind`: DNS publication boundary with a file-backed publisher
- `sdns-store`: PostgreSQL store plus an in-memory test double
- `sdns-audit`: signed audit ledger for asset and DNS events
- `sdns-core`: semantic record model, naming rules, and metadata merge logic
- `sdns-dhcp`: fingerprinting, role templates, and replacement detection
- `sdns-fathom`: importer for Fathom AssetDB-style PostgreSQL state
- `sdns-common`: shared IDs, config loading, auth roles, and errors
- `apps/web`: React + Tailwind operator console for records, graph, ops, and
  audit views

## Development

Run the standard quality checks locally:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The repository includes a pinned toolchain in `rust-toolchain.toml` and a CI
workflow in `.github/workflows/ci.yml` that runs the same checks on pushes and
pull requests.

## Deployment

The deployment assets live in the multi-stage [Dockerfile](Dockerfile), the
runtime config generator in
[deploy/docker-entrypoint.sh](deploy/docker-entrypoint.sh), and the full stack
definition in [deploy/server-compose.yaml](deploy/server-compose.yaml).

## License

This repository is licensed under the GNU Affero General Public License v3.0.
See [LICENSE](LICENSE). The bundled tokens and passwords are development
defaults only and must be replaced for any shared or production deployment.
