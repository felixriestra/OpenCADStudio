# Homebrew cask

[`mac2cam.rb`](mac2cam.rb) is a [Homebrew Cask](https://docs.brew.sh/Cask-Cookbook)
for installing the macOS (Apple Silicon) build.

## Install without a tap (directly from this file)

```bash
brew install --cask --no-quarantine \
  https://raw.githubusercontent.com/HakanSeven12/Mac2CAM/main/packaging/homebrew/mac2cam.rb
```

`--no-quarantine` is required: the app is ad-hoc signed but **not** Apple-notarised
(notarisation needs a paid Apple Developer ID), so without it Gatekeeper still
blocks the first launch.

## Publishing as a proper tap (recommended)

Create a separate GitHub repo named `homebrew-tap` under the same account, put a
copy of `mac2cam.rb` in its `Casks/` directory, then users can run:

```bash
brew install --cask --no-quarantine hakanseven12/tap/mac2cam
```

`brew upgrade` then keeps the app current automatically.

## Updating for a new release

Each release run prints the new `version` + `sha256` in the GitHub Actions job
summary (the **Emit Homebrew cask sha256** step). Paste those two lines into the
cask. To compute the digest manually:

```bash
shasum -a 256 Mac2CAM-vX.Y.Z-macos-arm64.dmg
```
