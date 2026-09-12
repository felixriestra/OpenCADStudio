cask "mac2cam" do
  version "0.4.8"
  sha256 "8fa92be6045c57ac1994bd20fbb2def7269e25c327b7e62cb59745977c66f421"

  url "https://github.com/HakanSeven12/Mac2CAM/releases/download/v#{version}/Mac2CAM-v#{version}-macos-arm64.dmg",
      verified: "github.com/HakanSeven12/Mac2CAM/"
  name "Mac2CAM"
  desc "CAD application for 2D drafting and 3D modeling that reads/writes DWG and DXF"
  homepage "https://github.com/HakanSeven12/Mac2CAM"

  # Only an Apple Silicon (arm64) build is published.
  depends_on arch: :arm64

  app "Mac2CAM.app"

  # The app is ad-hoc signed but not notarised (no paid Apple Developer ID).
  # Install with `--no-quarantine` so Gatekeeper does not block first launch:
  #   brew install --cask --no-quarantine packaging/homebrew/mac2cam.rb
  # or, once published to a tap:
  #   brew install --cask --no-quarantine hakanseven12/tap/mac2cam

  zap trash: [
    "~/Library/Application Support/Mac2CAM",
    "~/Library/Preferences/io.github.HakanSeven12.Mac2CAM.plist",
    "~/Library/Saved Application State/io.github.HakanSeven12.Mac2CAM.savedState",
  ]
end
