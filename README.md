English | [日本語](README.ja.md)

# Suisha

[Report a problem or suggest an idea](https://github.com/moriyama-dev/suisha/issues)

Suisha (水車, "water wheel") keeps the software on Ubuntu and macOS up to date by itself.
It brings the package managers you already use into one window, so you can search, install, update and remove apps without remembering which tool each one came from.
On a server, or over SSH, the same thing is one command: `suisha`.

| | Works with |
|---|---|
| **Ubuntu** 22.04 or later | **apt**, **snap**, **Flatpak** and **Homebrew** (on Linux) |
| **macOS** 13 Ventura or later | **Homebrew** (formulae and casks) and the **Mac App Store** (through [mas](https://github.com/mas-cli/mas)) |

Suisha is free and open source. It sends nothing about you or your computer anywhere, and every command it runs (`brew upgrade`, `apt-get upgrade` and so on) is shown to you.

![Updates in Suisha, with automatic updates turned on](docs/updates.png)

## What it does

- **Automatic updates.** Every hour unless you choose otherwise: every 3, 6 or 12 hours, every day, or one day a week. The system's own timer (systemd on Ubuntu, launchd on macOS) starts Suisha at that time, even when its window is closed, and updates everything. You get a notification when updates were installed or something failed. While the window is open, Suisha also looks for new updates every 30 minutes.
- **A check when the computer starts.** A minute or two after you log in, Suisha looks for updates and shows a notification when some are waiting. It is on from the start; turn off **Check for updates when the computer starts** in the automatic updates card, or run `suisha autostart off`. With automatic updates on, it runs one instead.
- **Update everything at once**, or only the packages you pick. Ubuntu's phased updates, which apt installs later on its own, are listed separately so that the count shows only what can be installed now. On macOS, apps from the Mac App Store are listed too, and their **Open App Store** button takes you straight to the App Store's Updates page.
- **Search every package manager at once** and install from the results.
- **See what you installed**, grouped by package manager, and remove what you no longer need. Dependencies and base system packages are left out, so they cannot be removed by accident.
- **Set up a new computer.** Save the list of your apps to a file, open it on a new or reinstalled computer, and Suisha installs whatever is missing in one go.
- **Notices that save a trip to the terminal** (Ubuntu): when Ubuntu needs a restart to finish updating, when packages are no longer needed (one button removes them), and when Flatpak has no app catalogue yet (one button adds Flathub).
- **History** of every install, removal and update, including the automatic ones. You choose how long entries are kept: 3, 30, 100 or 400 days, any number of days you type in, forever, or not at all. The default is 100 days.
- English and Japanese.

Suisha keeps no package catalog of its own. Everything comes from the package managers on your computer, so it shows exactly what they would.

![Searching all package managers at once](docs/search.png)

## On a server, or in a terminal

The `suisha` command does what the window does, without a desktop. It comes with the window, and on a server it installs on its own as the small `suisha-cli` package ([Install](#ubuntu)).

```console
$ suisha status
Suisha 0.6.1

Updates
  apt   7 updates waiting  61 installed
  snap  up to date         4 installed
  Install them with: suisha update

Automatic updates
  every day at 03:00
  Package managers: all
  Next run: 2026-11-25 03:03
  Last run: 2026-11-24 03:15
```

| | |
|---|---|
| `suisha status` | Waiting updates, the schedule, and what the system asks for (a restart, leftover packages) |
| `suisha updates` | List the updates that are waiting |
| `suisha update` | Install every update. `suisha update apt:git` updates only that package |
| `suisha on` | Update automatically: every hour, Suisha looks for updates in every package manager and installs them. `suisha off` stops it |
| `suisha autostart on`, `suisha autostart off` | Check for updates a little after the server starts (on unless turned off). The result is in `suisha status` |
| `suisha schedule set daily 03:00` | Update automatically at other times. Also `weekly sun 03:00` and `every 6h` |
| `suisha list`, `suisha search NAME` | What is installed; search every package manager |
| `suisha install apt:htop snap:btop`, `suisha remove htop` | Install and remove |
| `suisha cleanup` | Remove packages nothing needs any more (`apt autoremove`) |
| `suisha export apps.json`, `suisha setup apps.json` | Save the list of installed apps; install it on another server |
| `suisha history` | What was changed, newest first, including the automatic updates |
| `suisha self-update` | Update Suisha itself |
| `suisha report`, `suisha request` | Report a problem or suggest an improvement: prints a link to a new GitHub issue with the versions filled in |

- A package is written `MANAGER:NAME`, the way `suisha list` and `suisha search` print it.
- Before anything is changed, Suisha shows the commands it is about to run and asks. `--yes` skips the question and `--dry-run` only shows the commands.
- `--json` prints the result as JSON, for scripts. The exit code is 0 when it worked, 1 when it did not, and 2 when the command line could not be understood.
- Messages are in English or Japanese, following the system language.
- `suisha help` lists everything.

On a server nobody is logged in most of the time. `suisha on` and `suisha schedule set …` therefore also keeps your user's timer running while you are logged out (`loginctl enable-linger`), and the result of each run is in `suisha history` and in `journalctl --user -u suisha-auto-update`.

## Install

### macOS

macOS 13 Ventura or later, on Apple silicon or Intel.

**With Homebrew (recommended: Suisha then keeps itself up to date)**

```sh
brew tap moriyama-dev/suisha https://github.com/moriyama-dev/suisha
brew install --cask moriyama-dev/suisha/suisha
```

To also see and update apps from the Mac App Store, install mas:

```sh
brew install mas
```

**From a downloaded disk image**

Download `suisha-<version>-macos-universal.dmg` from the [Releases](https://github.com/moriyama-dev/suisha/releases) page, open it and drag **Suisha** to **Applications**.

Suisha is not notarized by Apple, so the first time macOS says it cannot check the app. Open **System Settings → Privacy & Security** and click **Open Anyway** next to the message about Suisha. Or run this once in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Suisha.app
```

A copy installed this way cannot update itself; Suisha shows a notice with a link to the new version instead. Installing with Homebrew avoids both steps.

### Ubuntu

Ubuntu 22.04 or later, x64.

**From the apt repository**

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://moriyama-dev.github.io/suisha/suisha-archive-keyring.gpg | sudo tee /etc/apt/keyrings/suisha-archive-keyring.gpg > /dev/null
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/suisha-archive-keyring.gpg] https://moriyama-dev.github.io/suisha stable main" | sudo tee /etc/apt/sources.list.d/suisha.list > /dev/null
sudo apt update
sudo apt install suisha        # a computer with a desktop: the window and the command
sudo apt install suisha-cli    # a server: only the command, no desktop libraries
```

Install one of the two. `suisha` brings `suisha-cli` with it.

**From downloaded packages**

Download `suisha-cli-<version>-linux-amd64.deb` and, for the window, `suisha-<version>-linux-amd64.deb` from the [Releases](https://github.com/moriyama-dev/suisha/releases) page and run:

```sh
sudo apt install ./suisha-cli-0.6.1-linux-amd64.deb ./suisha-0.6.1-linux-amd64.deb   # with a desktop
sudo apt install ./suisha-cli-0.6.1-linux-amd64.deb                                   # on a server
```

Either way, Suisha is updated from then on: the package adds Suisha's apt repository (`/etc/apt/sources.list.d/suisha.list`) if it is not there yet.
The repository is signed with the key `5E50 CE53 AC3D C290 5D94  3D67 F3A3 49A9 8664 021E`.

## How automatic updates work

Suisha does not stay running in the background. When you turn automatic updates on, it registers a timer with the system:

- **Ubuntu:** a systemd user timer (`~/.config/systemd/user/suisha-auto-update.timer`). If the computer was off at that time, the update runs after you next log in. When the schedule is set from a terminal without a desktop (a server), Suisha also turns on lingering for your user, so the timer runs while nobody is logged in.
- **macOS:** a launchd agent (`~/Library/LaunchAgents/dev.moriyama.suisha.auto-update.plist`). If the Mac was asleep, the update runs after it wakes up. Its output goes to `~/Library/Logs/Suisha/auto-update.log`.

Package servers do not announce new versions, so Suisha checks on a schedule; once an hour is the shortest interval, to keep the load low.

The timer runs `suisha --auto-update`, which updates every package manager you selected, writes the result to the history and shows a notification.
Turning automatic updates off removes the timer.

### The check at startup

This does not keep Suisha running either. It is on from the start, and the switch (or `suisha autostart on|off`) only changes a setting:

- **Ubuntu:** the `suisha-cli` package installs a systemd user timer for every user, `/usr/lib/systemd/user/suisha-startup-check.timer`. It runs `suisha --startup-check` two minutes after systemd starts for your user: when you log in, or when the computer starts if lingering is on (a server where you ran `suisha on`, or `suisha autostart on --linger`). System accounts such as the login screen are left out.
- **macOS:** a launchd agent that runs at login (`~/Library/LaunchAgents/dev.moriyama.suisha.startup-check.plist`). It waits a minute before checking. Its output goes to `~/Library/Logs/Suisha/startup-check.log`.

The check counts the waiting updates, notes the result for `suisha status`, and shows a notification when there are any. With automatic updates on, it runs an automatic update instead. It installs nothing on its own.

Mac App Store apps are not part of Suisha's automatic updates: the App Store updates them by itself when **Automatic Updates** is turned on in its settings.

### Passwords on macOS

Homebrew needs no administrator password for most apps. A few casks run an installer that needs one; when you update or install such an app from the window, macOS shows a password dialog from Suisha (Homebrew asks through `sudo -A`). The password goes straight to `sudo`; Suisha does not keep it. Scheduled updates never ask, so such an app is reported as failed in the history until you update it from the window.

`mas` needs administrator rights to install and update App Store apps and asks for the password in a terminal, which a window cannot answer. So Suisha leaves installing and updating to the App Store itself: **Open in App Store** opens the app's page and **Open App Store** opens the Updates page.

### Passwords on Ubuntu

apt and snap need administrator rights. The window asks for them through the standard system password dialog (polkit), and the `suisha` command through `sudo` in the terminal. Run as root, the command asks nothing.

- **When you install, remove or update by hand**, the system asks for your password. It is remembered for a few minutes.
- **For automatic updates**, nobody is there to type a password. When you turn automatic updates on (or press **Register password** on the automatic updates card), Suisha asks for your password once and lets your user run one fixed helper, `/usr/lib/suisha/suisha-auto-update`, without a password. That helper takes no arguments and only runs `apt-get update`, `apt-get upgrade` and `snap refresh`. Nothing else becomes password-free.
  - From the window, this is a polkit rule.
  - From the command, it is one line in `/etc/sudoers.d/suisha-auto-update-<your user id>`, checked with `visudo` before it is put in place.
- Turning automatic updates off asks for your password once more and removes the permission. Removing Suisha removes it too.

Flatpak and Homebrew work without administrator rights.

## Updating Suisha itself

- **With everything else.** On Ubuntu, Suisha is an apt package from its own repository, so "Update all", automatic updates and a plain `sudo apt upgrade` all update it. On macOS, Suisha is a Homebrew cask, so "Update all", automatic updates and `brew upgrade` update it.
- **From the button in the top right corner.** It shows Suisha's version and turns into a notice when a new version is out. **Update Suisha now** installs it, and **Restart Suisha** starts the new version. In a terminal, `suisha self-update` does the same.
- **Automatically.** Turn on **Update Suisha automatically** in that same place, or run `suisha self-update --auto on`. Once a day the system runs `suisha --self-update`.
  - On Ubuntu, a systemd user timer does this. Turning it on asks for your password once and lets your user run one fixed helper, `/usr/lib/suisha/suisha-self-update`, without a password. That helper takes no arguments and only refreshes Suisha's own apt source and upgrades the `suisha-cli` and `suisha` packages.
  - On macOS, a launchd agent runs `brew upgrade --cask moriyama-dev/suisha/suisha`. No password is needed. This needs Suisha installed with Homebrew.

![The button in the top right corner, with a new version of Suisha available](docs/suisha-update.png)

To know about new versions, the window asks GitHub for Suisha's latest release when it starts and every 6 hours while it is open; the command asks only when you run `suisha self-update`. Only the public release information is requested; nothing about you or your computer is sent.

## The saved app list

**Set up → Save list…** in the window, or `suisha export apps.json` in a terminal, writes a JSON file like this:

```json
{
  "format": "suisha-setup",
  "version": 1,
  "os": "linux",
  "exportedAt": "2026-11-20T02:20:07Z",
  "packages": [
    { "manager": "apt", "id": "git" },
    { "manager": "snap", "id": "code", "classic": true },
    { "manager": "flatpak", "id": "org.gimp.GIMP", "name": "GNU Image Manipulation Program", "origin": "flathub", "scope": "system" }
  ]
}
```

It contains package names only: no settings, files or personal data.
Lists are for moving between computers with the same system: a list saved on macOS (`"os": "macos"`, with `brew`, `brew-cask` and `mas` entries) opens on another Mac, and a list saved on Ubuntu opens on another Ubuntu computer.
On a Mac, App Store apps from a list are opened in the App Store, one page each, to be installed there.
In a terminal, `suisha setup apps.json` shows what the list has and this computer lacks, and installs it.

## Build from source

The code is one Cargo workspace:

| | |
|---|---|
| `crates/core` | Reading and driving the package managers, the schedule, the history and the settings |
| `crates/cli` | The `suisha` command |
| `src-tauri`, `src` | The window (Tauri and Svelte) |
| `packaging/linux` | The root helpers, the polkit policy and the package scripts |

The command needs only Rust (stable):

```sh
git clone https://github.com/moriyama-dev/suisha.git
cd suisha
cargo run -p suisha-cli -- status                  # run the command
cargo build --release -p suisha-cli                # build it
scripts/build-cli-deb.sh                           # Ubuntu: build the suisha-cli .deb
```

For the window you also need Node.js 22 and, on Ubuntu, the WebKitGTK development files:

```sh
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev build-essential
```

On macOS, the Xcode Command Line Tools (`xcode-select --install`).

```sh
npm ci
npm run tauri dev                                  # run in development mode
npm run tauri build                                # Ubuntu: build the .deb
npm run tauri build -- --bundles app,dmg           # macOS: build the app and the disk image
```

Tests:

```sh
npm run check                  # type-check the interface
cargo test --workspace         # parsers, scheduling (systemd and launchd), setup files, the command line, input checks
```

Suisha is built with [Tauri](https://tauri.app/) (Rust) and [Svelte](https://svelte.dev/).
All package manager commands run on the Rust side, without a shell, and package names are checked before they are passed on.

## License

[MIT](LICENSE)
