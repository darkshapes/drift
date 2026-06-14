# Plan: Modify Script Discovery to match scripts ending with ati_plug

## Stage 1: Update script entrypoint discovery logic

Modify `drift-node/src/script_discovery.rs` to find scripts ending with `ati_plug` instead of exact matches.

| Step | Description                                                                                      | %   | Line |
| :--- | :----------------------------------------------------------------------------------------------- | :-- | :--- |
| 1.1  | Modify `find_ati_plug` to iterate over `project.scripts` keys and check `.ends_with("ati_plug")` | 0%  |      |
| 1.2  | Modify `find_ati_plug` to iterate over `tool.uv.scripts` keys and check `.ends_with("ati_plug")` | 0%  |      |

- [ ] Stage 1 Complete

**Completion State:** The `find_ati_plug` function in `drift-node/src/script_discovery.rs` successfully returns the value of any script key that ends with `ati_plug` in either the `[project.scripts]` or `[tool.uv.scripts]` sections of `pyproject.toml`.
