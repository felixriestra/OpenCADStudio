# Local patches on top of iced-rs/iced

This is a full clone of upstream `iced-rs/iced`, checked out at the exact
rev Mac2CAM previously depended on directly via git
(`23604ff22ab0aad9e00b9327cb7b8546ed84db39`), vendored locally instead of
fetched from GitHub so the patches below can travel with this repo without
needing a fork-and-push to an external remote.

Root `Cargo.toml` points `iced`, `iced_core`, and `iced_runtime` at this
checkout via `path = "vendor/iced[...]"`, and `[patch.crates-io]` redirects
`iced_core`/`iced_widget` (pulled transitively by `iced_aw` from crates.io)
here too, so the whole dependency graph resolves to one consistent, patched
copy.

## Patches

1. **`core/src/window/settings/macos.rs`** — added
   `PlatformSpecific::tabbing_identifier: Option<String>` (and dropped the
   `Copy` derive, since `Option<String>` isn't `Copy`; nothing relied on it).
2. **`winit/src/conversion.rs`** — forwards that field to winit's existing
   `WindowAttributesExtMacOS::with_tabbing_identifier`.

Together these let Mac2CAM give a secondary window (e.g. the CAM Tool
Database) its own `NSWindow` tabbing identifier, so macOS's automatic
window tabbing (System Settings → Desktop & Dock → "Prefer tabs when
opening documents") can't silently merge it into a tab of the main window.
Every other window is unaffected — the field defaults to `None`, i.e. the
same behavior as before this patch.

## Updating the pinned rev later

If Mac2CAM's iced pin ever moves forward, re-apply these two small diffs by
hand (`git diff` against `23604ff2` will show exactly what changed here) —
don't try to `git pull`/rebase this checkout in place, since it's a plain
clone, not a fork with a proper upstream remote tracking relationship.
