# Homebrew cask for Suisha on macOS. The Release workflow updates version and sha256.
#   brew tap moriyama-dev/suisha https://github.com/moriyama-dev/suisha
#   brew install --cask moriyama-dev/suisha/suisha
cask "suisha" do
  version "0.5.2"
  sha256 "04da3f3ea88df131d72210192490ab14f99c9fd049310b3d2e12c34ae0deb1a8"

  url "https://github.com/moriyama-dev/suisha/releases/download/#{version}/suisha-#{version}-macos-universal.dmg"
  name "Suisha"
  desc "Keep Homebrew and App Store apps up to date from one window"
  homepage "https://github.com/moriyama-dev/suisha"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: :ventura

  app "Suisha.app"

  # The app is not notarized: remove the quarantine flag so that it opens
  # without the "cannot be verified" warning.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{appdir}}/Suisha.app"]
  end

  # The scheduled jobs stay through upgrades (Suisha updates itself with
  # `brew upgrade`), so they are only removed together with the settings.
  zap launchctl: [
        "dev.moriyama.suisha.auto-update",
        "dev.moriyama.suisha.self-update",
      ],
      trash:     [
        "~/Library/Application Support/suisha",
        "~/Library/Caches/dev.moriyama.suisha",
        "~/Library/LaunchAgents/dev.moriyama.suisha.auto-update.plist",
        "~/Library/LaunchAgents/dev.moriyama.suisha.self-update.plist",
        "~/Library/Logs/Suisha",
        "~/Library/WebKit/dev.moriyama.suisha",
      ]
end
