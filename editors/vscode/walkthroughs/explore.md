## Edit Global Configuration

`default.yml` is the entry point for your Fleet GitOps repository. It defines:

- **policies** — Compliance checks that run on every host
- **reports** — Scheduled osquery queries that collect data
- **labels** — Dynamic groups for targeting policies and software
- **org_settings** — Server-wide configuration
- **controls** — MDM profiles, OS updates, and scripts
- **software** — Packages, App Store apps, and Fleet-maintained apps

Start typing a top-level key and the extension will suggest valid fields with documentation.
