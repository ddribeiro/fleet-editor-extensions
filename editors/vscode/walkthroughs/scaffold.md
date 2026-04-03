## Scaffold a Fleet GitOps Repository

The **Fleet: Get Started** command creates a ready-to-use directory structure:

```
your-repo/
  default.yml                  # Org-level config
  fleets/
    workstations/
      default.yml              # Fleet-specific config
  lib/
    policies/                  # Shared policies
    reports/                   # Shared queries/reports
    labels/                    # Shared labels
    scripts/                   # Shared scripts
```

- `default.yml` — Global policies, reports, labels, and org settings
- `fleets/` — One subdirectory per fleet, each with its own `default.yml`
- `lib/` — Shared resources referenced via `path:` from any config file
