# Releasing & archiving Semantic DNS

This runbook turns a tagged release into a citable, permanently archived
artifact across GitHub → Zenodo → Software Heritage → TechRxiv.

## Metadata files in this repo

| File             | Consumed by                                  |
|------------------|----------------------------------------------|
| `CITATION.cff`   | GitHub "Cite this repository", Zenodo         |
| `.zenodo.json`   | Zenodo (overrides CFF-derived metadata)       |
| `codemeta.json`  | Software Heritage, general tooling            |

### Identity fields (done)

Author is set to **Daniel Caudle**, affiliation **Eris Securitas**, ORCID
`0009-0002-2330-8203` in `CITATION.cff`, `.zenodo.json`, and
`codemeta.json`. Zenodo freezes metadata at publish time, so verify these
are correct before the first tagged release.

Validate the CFF before tagging:

```sh
pipx run cffconvert --validate
```

## One-time: connect the services

1. **Zenodo**: sign in at <https://zenodo.org> with GitHub, open
   *Account → GitHub*, and flip the toggle ON for
   `erisforge/semantic-dns`. (Only releases created *after* the
   toggle is on are archived.)
2. **Software Heritage**: no account needed; the repo URL is what gets
   archived (see per-release step below).
3. **TechRxiv**: create an account at <https://www.techrxiv.org> for when
   the write-up is ready.

## Per-release steps

1. Bump `version` in `Cargo.toml`, `CITATION.cff`, and `codemeta.json`.
2. Set `date-released` in `CITATION.cff` to the release date.
3. Commit, then tag and push:
   ```sh
   git tag -a v0.1.0 -m "Semantic DNS 0.1.0"
   git push origin v0.1.0
   ```
4. Create the GitHub Release from that tag (UI or `gh release create v0.1.0`).
5. **Zenodo** auto-ingests within minutes. Confirm the deposition, then
   record both DOIs:
   - *concept DOI* — always resolves to the latest version; cite this in
     papers and in the README badge.
   - *version DOI* — pins exactly this release; use for reproducibility.
6. **Software Heritage** — trigger an explicit save so the exact tag is
   captured promptly instead of waiting for the crawler:
   ```sh
   curl -X POST \
     https://archive.softwareheritage.org/api/1/origin/save/git/url/https://github.com/erisforge/semantic-dns/
   ```
   Then resolve the snapshot/release SWHID at
   <https://archive.softwareheritage.org> for the tagged commit.
7. Add/refresh the DOI badge in `README.md` (Zenodo provides the Markdown
   on the deposition page — use the concept DOI).

## TechRxiv (when the paper is ready)

- Reference the Zenodo **concept DOI** in the paper's code/data
  availability section; optionally cite a Software Heritage **SWHID** for
  an exact code snapshot.
- Upload the manuscript to TechRxiv; it mints its own preprint DOI and
  can later be linked to a journal version.

## Order of operations (summary)

```
fix metadata → bump versions → tag → GitHub Release
   → Zenodo DOI (auto) → Software Heritage save (curl) → README badge
   → cite concept DOI in TechRxiv manuscript → post preprint
```
