# Show-Default Legend Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a layer's `metadata.legend` opt out of rendering the catch-all default entry produced by `match`/`case` expressions, so unwanted default rows stop reserving blank space in the SVG.

**Architecture:** A new `get_show_default(layer) -> Result<bool, LegendError>` helper in `src/common.rs` reads `metadata.legend.show-default` (boolean, default `true`). `parse_match` and `parse_case` — the single shared point where `Vec<(label, color)>` entries are assembled for every layer type — gate their existing "push the default entry" block on this flag. No renderer (`fill.rs`, `circle.rs`, `line.rs`, `fill_extrusion.rs`, `symbol.rs`) needs to change.

**Tech Stack:** Rust, serde_json, existing test module in `src/common.rs` (`#[cfg(test)] mod tests`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-15-show-default-legend-entry-design.md`.
- `show-default` default is `true` when absent — must not change behavior of any existing style.json without the key (regression tests required).
- Applies only to `match` and `case`. `interpolate` and `step` are out of scope — do not touch `parse_interpolate` or `parse_step`.
- No signature changes to `parse_match`, `parse_case`, `parse_expression`, or any renderer function.
- Follow existing code style: this codebase already uses `if cond && let Some(x) = ...` let-chains (see `parse_case`'s existing default block and `format_condition`) — use the same style for the new conditions rather than nested `if`.
- Run `cargo fmt` and `cargo clippy` before each commit (project requirement per `CLAUDE.md`).

---

### Task 1: `get_show_default` helper in `common.rs`

**Files:**
- Modify: `src/common.rs` (insert new function after `get_layer_default_label`, which ends at line 131)
- Test: `src/common.rs` (`#[cfg(test)] mod tests` block, already at the bottom of the file)

**Interfaces:**
- Consumes: `Layer` struct, `get_legend_object(layer: &Layer) -> Result<Option<&Map<String, Value>>, LegendError>` (already defined at line 95).
- Produces: `pub fn get_show_default(layer: &Layer) -> Result<bool, LegendError>` — used by Task 2 and Task 3.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `src/common.rs` (near the other `get_*` tests, e.g. after `test_parse_match_with_downcase`):

```rust
    #[test]
    fn test_get_show_default_absent_defaults_true() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        assert!(get_show_default(&layer).unwrap());
    }

    #[test]
    fn test_get_show_default_explicit_true() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"show-default": true}}
        }))
        .unwrap();
        assert!(get_show_default(&layer).unwrap());
    }

    #[test]
    fn test_get_show_default_explicit_false() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"show-default": false}}
        }))
        .unwrap();
        assert!(!get_show_default(&layer).unwrap());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_get_show_default`
Expected: compile error — `get_show_default` is not defined.

- [ ] **Step 3: Implement `get_show_default`**

Insert into `src/common.rs` directly after `get_layer_default_label` (after the closing `}` at line 131, before `get_custom_labels` at line 133):

```rust
pub fn get_show_default(layer: &Layer) -> Result<bool, LegendError> {
    let legend = get_legend_object(layer)?;
    let show_default = legend
        .and_then(|l| l.get("show-default"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok(show_default)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_get_show_default`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src/common.rs
git commit -m "feat: add get_show_default helper for metadata.legend"
```

---

### Task 2: Wire `show-default` into `parse_match`

**Files:**
- Modify: `src/common.rs`, function `parse_match` (currently lines 476-537), default-push block at lines 527-534
- Test: `src/common.rs` (`mod tests`)

**Interfaces:**
- Consumes: `get_show_default(layer: &Layer) -> Result<bool, LegendError>` from Task 1.
- Produces: no new public interface — `parse_match`'s existing signature and return type (`Result<Vec<(String, String)>, LegendError>`) are unchanged; only the entries it produces change when `show-default` is `false`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/common.rs`:

```rust
    #[test]
    fn test_parse_match_show_default_false_omits_default_entry() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"show-default": false}}
        }))
        .unwrap();
        let expr = json!([
            "match", ["get", "tipo"],
            "bosque", "#228B22",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ("bosque".to_string(), "#228B22".to_string()));
    }

    #[test]
    fn test_parse_match_show_default_true_keeps_default_entry() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"show-default": true, "default": "Otros"}}
        }))
        .unwrap();
        let expr = json!([
            "match", ["get", "tipo"],
            "bosque", "#228B22",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1], ("Otros".to_string(), "#cccccc".to_string()));
    }

    #[test]
    fn test_parse_match_show_default_absent_keeps_default_entry() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!([
            "match", ["get", "tipo"],
            "bosque", "#228B22",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_match_show_default_false_ignores_extra_custom_label() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {
                "legend": {
                    "show-default": false,
                    "custom-labels": ["Bosque", "Otros"]
                }
            }
        }))
        .unwrap();
        let expr = json!([
            "match", ["get", "tipo"],
            "bosque", "#228B22",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Bosque");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_parse_match_show_default`
Expected: `test_parse_match_show_default_false_omits_default_entry` and
`test_parse_match_show_default_false_ignores_extra_custom_label` FAIL (result has 2
entries instead of 1, because the default is still unconditionally pushed). The other
two tests already pass since they describe current behavior.

- [ ] **Step 3: Gate the default-push block on `get_show_default`**

In `src/common.rs`, in `parse_match`, replace:

```rust
    if let Some(default_color) = arr.last().and_then(|v| v.as_str()) {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}
```

with:

```rust
    if get_show_default(layer)?
        && let Some(default_color) = arr.last().and_then(|v| v.as_str())
    {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_parse_match`
Expected: all `test_parse_match_*` tests pass (including the pre-existing
`test_parse_match_with_downcase`).

- [ ] **Step 5: Commit**

```bash
git add src/common.rs
git commit -m "feat: support show-default:false in match expressions"
```

---

### Task 3: Wire `show-default` into `parse_case`

**Files:**
- Modify: `src/common.rs`, function `parse_case` (currently lines 543-583), default-push block at lines 571-580
- Test: `src/common.rs` (`mod tests`)

**Interfaces:**
- Consumes: `get_show_default(layer: &Layer) -> Result<bool, LegendError>` from Task 1.
- Produces: no new public interface — same as Task 2, but for `parse_case`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/common.rs`:

```rust
    #[test]
    fn test_parse_case_show_default_false_omits_default_entry() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"show-default": false}}
        }))
        .unwrap();
        let expr = json!([
            "case",
            ["has", "nombre"], "#ff0000",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ("has nombre".to_string(), "#ff0000".to_string()));
    }

    #[test]
    fn test_parse_case_show_default_true_keeps_default_entry() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"default": "Otros"}}
        }))
        .unwrap();
        let expr = json!([
            "case",
            ["has", "nombre"], "#ff0000",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1], ("Otros".to_string(), "#cccccc".to_string()));
    }

    #[test]
    fn test_parse_case_show_default_absent_keeps_default_entry() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!([
            "case",
            ["has", "nombre"], "#ff0000",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 2);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_parse_case_show_default`
Expected: `test_parse_case_show_default_false_omits_default_entry` FAILS (result has 2
entries instead of 1). The other two already pass.

- [ ] **Step 3: Gate the default-push block on `get_show_default`**

In `src/common.rs`, in `parse_case`, replace:

```rust
    if arr.len().is_multiple_of(2)
        && let Some(default_color) = arr.last().and_then(|v| v.as_str())
    {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}
```

with:

```rust
    if get_show_default(layer)?
        && arr.len().is_multiple_of(2)
        && let Some(default_color) = arr.last().and_then(|v| v.as_str())
    {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_parse_case`
Expected: all `test_parse_case_*` tests pass (including the pre-existing
`test_parse_case_basic` and `test_parse_case_with_custom_labels`).

- [ ] **Step 5: Commit**

```bash
git add src/common.rs
git commit -m "feat: support show-default:false in case expressions"
```

---

### Task 4: Full test suite, docs, and changelog

**Files:**
- Modify: `README.md` (metadata.legend key table, currently lines 146-150)
- Modify: `CHANGELOG.md` (`## [Unreleased]` → `### Added`, currently lines 5-11)

**Interfaces:**
- Consumes: nothing new — this task only updates docs and runs the full verification suite.
- Produces: nothing consumed by later tasks (this is the final task).

- [ ] **Step 1: Run the full test suite and lints**

Run: `cargo test`
Expected: all tests pass, including the 10 new tests from Tasks 1-3.

Run: `cargo fmt --check`
Expected: no diff. If there is a diff, run `cargo fmt` and re-check.

Run: `cargo clippy`
Expected: no warnings.

- [ ] **Step 2: Update `README.md`**

In the `metadata.legend` key table (`README.md:146-150`), change:

```markdown
| Key | Type | Description |
|-----|------|-------------|
| `label` | string | Title for the legend entry. Falls back to the layer `id`. |
| `default` | string | Label for the expression's fallback/default color. |
| `custom-labels` | array of strings | Labels for each stop or case in the expression, in order. |
```

to:

```markdown
| Key | Type | Description |
|-----|------|-------------|
| `label` | string | Title for the legend entry. Falls back to the layer `id`. |
| `default` | string | Label for the expression's fallback/default color. |
| `custom-labels` | array of strings | Labels for each stop or case in the expression, in order. |
| `show-default` | boolean | Whether to include the `match`/`case` fallback/default entry in the legend. Defaults to `true`. Set to `false` to omit it entirely (no reserved row), instead of hiding it with an empty `default` label. |
```

- [ ] **Step 3: Update `CHANGELOG.md`**

In `CHANGELOG.md`, under `## [Unreleased]` → `### Added` (`CHANGELOG.md:7-11`), add a
bullet after the existing `fill-pattern` entry:

```markdown
- `metadata.legend` gains a `show-default` boolean (for `match`/`case` expressions).
  Set it to `false` to omit the fallback/default entry from the legend entirely,
  instead of the previous workaround of an empty `default` label that still reserved
  a blank row.
```

- [ ] **Step 4: Verify docs render correctly**

Run: `grep -n "show-default" README.md CHANGELOG.md`
Expected: one match in each file, matching the text added in Steps 2-3.

- [ ] **Step 5: Commit**

```bash
git add README.md CHANGELOG.md
git commit -m "docs: document show-default metadata.legend option"
```
