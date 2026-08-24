# GSA/dcat-us Pull Request #120

Retrieved verbatim via `gh api repos/GSA/dcat-us/pulls/120` and
`gh api repos/GSA/dcat-us/pulls/120/files` (GitHub REST API v3), 2026-08-25.
Source: <https://github.com/GSA/dcat-us/pull/120>

## Metadata

- **Title:** Move aside outdated files and emphasize the JSON Schema
- **Author:** neilmb (Neil Martinsen-Burrell)
- **Merged:** 2026-04-03T17:19:28Z, by James Brown (james.c.brown@gsa.gov)
- **Merge commit:** `6de1ee3fc34f550ad741184bd45f5e35825bcea7`
- **Base repository:** GSA/dcat-us
- **Files changed:** 274
- **Additions:** 65
- **Deletions:** 4779

## Body (verbatim)

> This addresses #44 by focusing on the JSON Schema expression of DCAT-US v3.0.
>
> Agency metadata catalog producers want a single approved format for their
> metadata catalogs. JSON formatted catalog files are universally used
> following DCAT-US v1.1 and JSON Schema is the natural way to express the
> schema of information that should be contained in those catalogs.
>
> Old information might still be useful for context, so we store it away in
> a clearly-labelled `DEPRECATED` folder.

## Files with `status: removed` (not renamed into `DEPRECATED/`)

Per the GitHub API, every other file touched by this pull request has
`status: renamed` (moved into the `DEPRECATED/` tree, byte-identical). Exactly
two files have `status: removed`, deleted outright with no successor path
anywhere in the diff:

| File | Status | Additions | Deletions |
|---|---|---|---|
| `context/dcat-us-3.0.jsonld` | removed | 0 | 1317 |
| `shacl/dcat-us_3.0_shacl_shapes.ttl` | removed | 0 | 3435 |

These were the repository's only published JSON-LD `@context` document and
its only SHACL shapes file. Neither has been restored or replaced anywhere in
the repository as of commit `7a6e803` (the commit this corpus was built
against). The JSON Schema definitions that remain make no reference to
either file.
