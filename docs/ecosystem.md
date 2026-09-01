# Floem ecosystem

Official satellites (not part of floem 1.0 core promise):

| Satellite | Role | Depends on |
|-----------|------|------------|
| [flome-ui](../flome-ui) | Material / Cupertino / Fluent kit (`floem-elements`) | `floem ^1` |
| [floem-editor](../floem-editor) | Code/text editor views + editor-core | `floem` public API only |

## Compatibility matrix

| floem | flome-ui | floem-editor | Notes |
|-------|----------|--------------|-------|
| 0.2.x (pre-extract) | path/git | in-tree `editor` feature (removed) | historical |
| 0.2.x (industrial-1.0) | path/git | [floem-editor](../floem-editor) satellite | editor extracted |
| 1.0.0 | (fill on release) | (fill on release) | |

Update this table when tagging floem or publishing satellites.
