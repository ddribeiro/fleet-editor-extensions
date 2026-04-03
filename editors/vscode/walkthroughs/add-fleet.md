## Add a Fleet

Fleets let you apply different configurations to different groups of hosts. Each fleet has its own directory under `fleets/`:

```
fleets/
  workstations/
    default.yml      # Policies, reports, controls for workstations
  servers/
    default.yml      # Different config for servers
  contractors/
    default.yml      # Limited access fleet
```

Each fleet `default.yml` supports the same keys as the global config, plus a `name` field and `settings` for fleet-specific options.

Create a new directory under `fleets/` and add a `default.yml` to get started.
