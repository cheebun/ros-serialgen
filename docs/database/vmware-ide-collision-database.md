# VMware Virtual IDE Hard Drive Collision Database

Collision results for model `"VMware Virtual IDE Hard Drive"` — PVE's **default** `ide0` model string when no custom `model=` is set. Truncates to 16 bytes for hashing (`VMware Virtual I`), so it is **not interchangeable** with any other model string, including the SATA default (see [vmware-sata-collision-database.md](vmware-sata-collision-database.md)). Standard identity (`00000000000000000000`), marker `BDE8`, unless noted otherwise.

---

## 6G

| Serial | SOFTWARE ID | Verified |
|---|---|---|
| `1` | TI09-7WK3 | Y |

Serial `1` (i.e. `00000000000000000001` — leading zeros are optional, `ros-serialgen check` left-pads pure-digit serials automatically) is PVE's default `serial=` value when none is set — this is the "do nothing, just create a default 6G `ide0` disk" collision.
