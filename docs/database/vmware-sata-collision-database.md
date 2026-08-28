# VMware Virtual SATA Hard Drive Collision Database

Collision results for model `"VMware Virtual SATA Hard Drive"` — PVE's **default** `sata0` model string when no custom `model=` is set. Truncates to 16 bytes for hashing (`VMware Virtual S`), so it is **not interchangeable** with the IDE default (see [vmware-ide-collision-database.md](vmware-ide-collision-database.md)) even though `sata0` and `ide0` otherwise share the same SOFTWARE ID encoding — the truncated model bytes differ. Standard identity (`00000000000000000000`), marker `BDE8`, unless noted otherwise.

No collision search has been run yet for this model. Once one is, results go here, organized by disk size (see [vmware-ide-collision-database.md](vmware-ide-collision-database.md) for the expected layout).
