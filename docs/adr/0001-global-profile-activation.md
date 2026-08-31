# Use global profile activation and explicit projection

Profile activation is global and stored with each profile rather than in per-target marker state. Targets only describe projection directories and optional shell adapters; inventory commands never mutate targets, while `apply` is the sole persistent projection operation and isolated shells provide temporary per-session variation. This trades per-target persistent profile selection for a filesystem model that remains understandable when users edit profile directories directly and when external installers overwrite target links.
