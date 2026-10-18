# Satellite

Satellite expands the capabilities of the Orbit Speak and Orbit Player, letting them reach their full potential. It sets
them up so that you can use any Android app, not only Orbit's own, with a full Android screen reader driven from the
Orbit's own keys, and it fixes some problems in Orbit's software. It also lets you create or install third-party
self-voicing apps that speak for themselves and receive keys directly, just like Orbit's own apps do. You set it up
once, from a Mac, Windows or Linux computer, with this program, Satellite Setup.

Satellite is an independent project. It isn't made, supported or endorsed by Orbit Research, so please don't ask
Orbit Research for help with it.

## What Satellite gives you

- **Backtalk, with the Orbit's keys.** [Backtalk](https://github.com/trypsynth/backtalk) is a screen reader for Android,
  based on Google's TalkBack, which reads Android apps instead of Orbit's very limited Android screen reader. Satellite
  turns the Orbit's keypad into a braille display for Backtalk, so you can use those apps with the keys you already
  know: the Speak's braille keys and the Player's phone keypad (dots 1, 2, 3 and 7 on the left column, dots 4, 5, 6 and
  8 on the right column, space bar on all the buttons of the middle column). In Orbit's own apps, Orbit's screen reader
  keeps working as it always has, and Backtalk stays quiet.
- **A new home screen, the Satellite Launcher.** It works like Orbit's home screen, with the same keys, but shows
  every app on the device, in folders you can arrange yourself. You can also find an item by typing its first letters,
  and on the Player, open an item by its number.
- **Google Play.** Orbit's home screen hides the Play Store, and some Orbits come with it and Play Services turned off.
  Satellite turns them on where they are off, and shows the Play Store on its home screen, so you can easily install any
  Android app you want from Google Play.
- **Fixes for problems in Orbit's software.** Daily and weekly alarms ring on time, and Dismiss stops an alarm rather
  than snoozing it. After an Orbit update that changes the screen reader, the device restarts by itself, as it should.
- **Apps that speak for themselves.** Android apps made to speak through Orbit's screen reader can say so, and Orbit
  then leaves them to it, as it does with its own apps.
- **Updates on the device.** Satellite Updates, in the Tools folder, keeps Satellite up to date over Wi-Fi, without a
  computer.

## What you need

- An Orbit Speak or Orbit Player.
- A Mac, Windows or Linux computer, connected to the internet.
- A USB cable that carries data, not one that only charges.
- The Orbit's battery at least half charged, or the charger connected: setup restarts the Orbit two or three times.

The Orbit doesn't need Wi-Fi for setup: Satellite Setup downloads everything on the computer.

## Download

Download Satellite Setup for your computer:

- [Mac](https://github.com/emassey0135/satellite-setup/releases/latest/download/satellite-setup-macos.dmg)
  (Apple silicon and Intel)
- [Windows (x86_64)](https://github.com/emassey0135/satellite-setup/releases/latest/download/satellite-setup-windows-x64.exe)
  (most PCs), or
  [Windows on ARM64](https://github.com/emassey0135/satellite-setup/releases/latest/download/satellite-setup-windows-arm64.exe)
- [Linux (x86_64)](https://github.com/emassey0135/satellite-setup/releases/latest/download/satellite-setup-linux-x86_64)
  (most PCs), or
  [Linux on ARM64](https://github.com/emassey0135/satellite-setup/releases/latest/download/satellite-setup-linux-aarch64)

Satellite Setup isn't signed yet, so your computer may warn you the first time you open it.

- **Mac:** open the downloaded file, and copy Satellite Setup to Applications. If macOS says it can't check the app
  for malicious software, open System Settings, then Privacy and Security, and choose Open Anyway beside Satellite
  Setup.
- **Windows:** the download is the program itself; open it. If Windows says it protected your PC, choose More info,
  then Run anyway. It needs Microsoft's WebView2, which Windows 11 has, and Windows 10 almost always has; if it doesn't
  start, [install WebView2](https://developer.microsoft.com/microsoft-edge/webview2/).
- **Linux:** the download is the program itself. Let it run, with `chmod +x satellite-setup-linux-*`, then open it. It
  needs WebKitGTK 4.1, which GNOME desktops have: the `webkit2gtk-4.1` package on Arch Linux, `libwebkit2gtk-4.1-0` on
  Debian and Ubuntu, and `webkit2gtk4.1` on Fedora. It runs on distributions from about 2022 on, such as Ubuntu 22.04
  and Debian 12 and later, and on rolling distributions such as Arch Linux. On ARM computers, it uses your
  distribution's adb (the `android-tools` package on Arch Linux, `adb` on Debian and Ubuntu), as Google doesn't make
  one for them.

Satellite Setup is made to be used with a screen reader, and has been used with VoiceOver on the Mac, NVDA on Windows
and Orca on Linux. It says setup's progress as it goes.

## Setting up

1. Connect the Orbit to the computer with the USB cable, and open Satellite Setup. The first time, it downloads
   Android's tools, which it needs to talk to the Orbit.
2. Satellite Setup finds the Orbit by itself and says so. If the Orbit asks whether to allow USB debugging from this
   computer, choose Allow. On Linux, if it says the computer isn't allowed to use the Orbit yet, choose the button to
   allow it, and enter your password. On Windows, if it says Windows needs a driver, choose the button to install it.
3. Look over the choices, which are fine as they are for most people:
   - **Turn on Google Play.** Orbit turns off Google Play on some devices; with it on, you can install apps from the
     Play Store.
   - **Turn off Do Not Disturb,** which Orbit turns on.
   - **The Orbit's language,** if it isn't right, such as English (United States) rather than English (India).
   - **Files to use instead of downloading them,** such as a Backtalk build you want to try. You don't need any.
4. Choose Set up Satellite, and keep the Orbit connected until setup says it is complete. It takes about 6 minutes.

The Orbit does most of the work itself, and says what it is doing; Satellite Setup says the same on the computer.
Along the way, the Orbit restarts two or three times. Once the Satellite Launcher is the home screen, it only says that
setup is running until setup is done. Near the end, Backtalk asks two questions on the Orbit, which Orbit's screen
reader reads:

- whether the Orbit's keys may be its braille display: choose **Allow**;
- whether to install the Orbit step aside script, which keeps Backtalk quiet in Orbit's own apps: choose **Install**.

When setup says it is complete, the Satellite Launcher is the Orbit's home screen, and Backtalk is on.

## After setting up

- **Updates:** the Satellite Launcher checks for updates once a day, and says when there is one. Choose Satellite
  Updates, in the Tools folder, to check now, and to install them. Backtalk updates itself.
- **Orbit's updates** keep working as before, and Satellite carries on afterwards.
- **Connecting a computer you haven't used with the Orbit before:** the Orbit asks whether to allow USB debugging from
  it, in a question Backtalk reads. Choose Allow.

## Undoing Satellite

Satellite Setup can put the Orbit back as it was before Satellite was set up on it: connect the Orbit, and choose Undo
Satellite setup. When it is done, restart the Orbit with its power button.

Setup keeps a backup of the settings it changes, so that it can do this, and it is careful with the device in other
ways too: it checks the Orbit before it changes anything, and if the Orbit can't finish starting after setup, it turns
LSPosed, which Satellite's fixes run in, off at the next start, so that the Orbit starts as it came.

## Wireless debugging

You can connect the Orbit to Satellite Setup with wireless debugging instead of a cable, under "Connect with wireless
debugging instead". It's harder to turn on, though, since the Orbit's Android settings are hidden, and it turns off
whenever the Orbit restarts, so a cable is easier.

## The parts of Satellite

- [Satellite Setup](https://github.com/emassey0135/satellite-setup) (this program), which sets Satellite up from a
  computer.
- [The Satellite Launcher](https://github.com/emassey0135/satellite-launcher), the home screen, which also does the
  setup itself on the Orbit, and Satellite's updates.
- [The Satellite hook](https://github.com/emassey0135/satellite-hook), which works inside Orbit's own apps, through
  LSPosed: it lets Orbit's screen reader and Backtalk share the Orbit, gives Backtalk the Orbit's keys, and fixes
  Orbit's bugs.
- [Backtalk](https://github.com/trypsynth/backtalk), the screen reader, which is its own project.

## Building Satellite Setup

Satellite Setup is a [Tauri](https://tauri.app) app: its window is in `ui`, and the work is done in Rust, in
`src-tauri`. With Rust, and on Linux WebKitGTK 4.1's development package, install Tauri's command-line tool with
`cargo install tauri-cli --version "^2"`, then run `cargo tauri dev` to try it, or `cargo tauri build` to build it.
GitHub Actions builds each push for every system, and publishes the downloads.

## License

Satellite Setup is under the [Apache License 2.0](LICENSE).
