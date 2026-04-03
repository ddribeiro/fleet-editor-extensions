---
icon: lucide/list-checks
---

# Lint rules

Flint includes 18 built-in rules across four categories. Run `flint list-rules` to see them all.

## Structural rules

| Rule | Fixable | Description |
|---|---|---|
| `required-fields` | Yes | Ensures policies have `name` and `query` |
| `type-validation` | Yes | Validates field types (booleans, enums) |
| `structural-validation` | Yes | Unknown keys, misplaced keys, typo suggestions |
| `duplicate-names` | No | Detects duplicate names in collections |
| `self-reference` | No | Detects `path:` references to own file |

## Semantic rules

| Rule | Fixable | Description |
|---|---|---|
| `platform-compatibility` | No | osquery tables vs. declared platform (129 tables) |
| `query-syntax` | No | Basic SQL validation (SELECT, balanced quotes) |
| `label-targeting` | No | `labels_include_any` and `labels_include_all` mutual exclusivity |
| `label-membership` | No | dynamic requires query, manual requires hosts |
| `date-format` | Yes | Deadlines must be YYYY-MM-DD |
| `hash-format` | Yes | SHA256 must be 64 lowercase hex characters |
| `categories` | Yes | Software categories from supported set |
| `file-extension` | No | MDM profile and script extension checks |
| `path-reference` | No | `path` vs `paths` glob validation (Fleet 4.83+) |

## Security rules

| Rule | Fixable | Description |
|---|---|---|
| `security` | Yes | Hardcoded secrets in webhook URLs |
| `secret-hygiene` | Yes | Integration credentials must use `$VAR` or `op://` |

## Deprecation rules

| Rule | Fixable | Description |
|---|---|---|
| `deprecated-keys` | Yes | Version-gated warnings for Fleet renames |

Current deprecations (warnings since v4.80.1):

- `teams/` -> `fleets/` (directory)
- `team_settings` -> `settings` (key)
- `queries` -> `reports` (key)
- `no-team.yml` -> `unassigned.yml` (file)

## Inline suppression

Suppress a rule on a specific line:

```yaml
queries:  # flint: ignore [deprecated-keys]
```

Suppress all rules:

```yaml
queries:  # flint: ignore
```
