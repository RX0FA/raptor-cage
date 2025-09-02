<div align="center">
  <img src="assets/icon.png" />
  <h1>
    raptor-cage
  </h1>
  <p>
    Run games in a secure sandbox, various native and non-native titles are supported.
  </p>
  <img alt="Downloads" src="https://img.shields.io/github/downloads/RX0FA/raptor-cage/total?style=flat-square&label=DOWNLOADS&labelColor=0567ff&color=696969" />
  <img alt="Latest Release" src="https://img.shields.io/github/v/release/RX0FA/raptor-cage?style=flat-square&label=LATEST%20RELEASE&labelColor=0567ff&color=696969" />
  <img alt="AUR" src="https://img.shields.io/aur/version/raptor-cage-bin?style=flat-square&label=AUR&labelColor=0567ff&color=696969" />
</div>

## ⬇️ Installation

### ArchLinux

> ⚠️ It's recommended to have multilib enabled on `pacman.conf`

```bash
# Using paru.
paru -S raptor-cage-bin

# Manual clone.
git clone https://aur.archlinux.org/raptor-cage-bin.git
cd raptor-cage-bin
makepkg -sri
```

### Manual Installation

```bash
download_url="$(curl -sL 'https://api.github.com/repos/RX0FA/raptor-cage/releases/latest' | grep -E 'browser_download_url.+\.tgz' | grep -oP '"browser_download_url": "\K[^"]+')"
curl -L -o raptor-cage.tgz "$download_url"
tar xf raptor-cage.tgz
sudo install -Dm755 raptor-cage "/usr/local/bin/rcage"
```

## 💡 Usage

> ⚠️ Network access is denied by default

### Command Line Examples

```bash
# Run Windows game, runner and prefix paths are relative to Bottles data directory.
rcage run -r soda-9.0-1 -p my_prefix -d ~/games/some_game -b game.exe

# Run native binary, and pass custom parameters.
rcage run -d ~/games/some_game -b native_binary -- --param1

# Mount game path as read-write, mount installer path as read-only, then start interactive shell.
rcage run -r soda-9.0-1 -p my_prefix  -d ~/games/some_game:rw -v ~/installers:/installers:

# Mount game path as read-write, mount installer path as read-only, then start "setup.exe".
rcage run -r soda-9.0-1 -p my_prefix  -d ~/games/some_game:rw -v ~/installers:/installers: -b /installers/setup.exe
```

### `rcage run` Enum Parameters

* --network-mode:
  * `full_access`: no network restrictions at all.
  * `restricted_access`: restricts access to some network features such as DNS resolving and SSL certificates, however internet connection is still possible through direct IPs.
  * `no_access`: network access is completely blocked, this is the default value if no option is passed.
* --device-access:
  * `all`: sandboxed program will have access to all devices i.e., `/dev` is completely exposed inside the sandbox.
  * `minimal`: a limited amount of devices are exposed inside the sandbox i.e., GPU, gamepads, etc; this is the default value.
* --upscale-mode:
  * `none`: no upscaling applied, this is the default value.
  * `dlss`: enable NVIDIA DLSS, **support depends on the wine runner**, raptor-cage only configures the necessary flags.
  * `fsr`: enable FSR, it requires additional options separated by `:`, the command value should look like `fsr:mode:strength`. Mode can be one of `none`, `quality`, `balanced`, `performance` or `ultra`; strength is a value that goes from 0 to 5; (example command: `--upscale-mode=fsr:balanced:1`). **Support depends on the wine runner** being used.
* --sync-mode: one of `none`, `fsync` or `esync`. The default value depends on the runner being used.

## 📌 Frequently Asked Questions

* How to enable MangoHud?  
  Use the `-e MANGOHUD=1` parameter for games that use DXVK and VK3D, other games (OpenGL and WineD3D) may require to prepend `mangohud` before the binary (e.g., `mangohud wine game.exe`).
* What is the difference with Bottles?  
  Bottles is a GUI to manage Wine/Proton instances and their dependencies, and it runs under Flatpak; applications that are launched from Bottles have access to everything Bottles has access to (you can see what can Bottles access [here](https://github.com/flathub/com.usebottles.bottles/blob/master/com.usebottles.bottles.yml#L9)), raptor-cage launches applications with a restricted sandbox by default, and allows the user to adjust permissions independently.
* Do I need Bottles in order to use raptor-cage?  
  No, Bottles is not needed, although is highly recommended in order to manage Wine/Proton versions and dependencies. If you don't want to use Bottles, you can download any Wine/Proton version you like, extract it anywhere and choose the respective path when running raptor-cage (`-r`).
* What is the difference with Bubblewrap?  
  Bubblewrap (bwrap) is used under the hood by raptor-cage, you could use bwrap directly too, however it would require careful configuration of dozens of parameters.
* Do I need Steam in order to use raptor-cage?  
  Not at all, raptor-cage objective is to allow the user to run games in a sandbox without relying on closed-source or corporate launchers/tools.
* You say that Steam is not required, but I still need to install `steam-native-runtime` on ArchLinux  
  The `steam-native-runtime` package on ArchLinux includes a lot of dependencies that Wine/Proton require to run games, it's used as a convenience shortcut to bring the necessary dependencies into your system, you can avoid installing `steam-native-runtime` by using the raptor-cage binary (non-package version) and install the dependencies yourself.
* Why do I have Steam icons on ArchLinux?  
  `steam-native-runtime` will be installed as a dependency of raptor-cage, if you want to avoid such icons, ignore the respective files on `pacman.conf`
  ```conf
  # /etc/pacman.conf
  NoExtract   = usr/bin/steam usr/bin/steam-runtime usr/bin/steamdeps usr/share/applications/steam.desktop
  NoExtract   = usr/bin/steam-native usr/share/applications/steam-native.desktop
  ```
* Do I still need `steam-native-runtime` on Manjaro?  
  Yes, even though Manjaro includes more dependencies than regular ArchLinux (which helps in many cases), if `steam-native-runtime` is not installed, there will still be some games that will just freeze with no explanation, or sometimes Wine/Proton will report that a dependency (like `libvulkan1.so`) is missing despite that not being the case.

## 🔥 Troubleshooting

> Recommended read https://wiki.archlinux.org/title/Steam/Troubleshooting#Steam:_An_X_Error_occurred

* **failed to load driver: nouveau:** make sure to have 32-bit libraries installed i.e., `lib32-nvidia-utils`

## ⚙️ Development

### Maintenance

```bash
# Check for dependency vulnerabilities.
cargo audit

# Perform minor dependency updates (Cargo.lock).
cargo update

# Check for updates (Cargo.toml).
cargo upgrade --dry-run
```

### TODOs

#### General

* Some games (like HC2, DXM) create a detached sub-process, since we are using `--die-with-parent`, said games will not run when executed directly (with `-b` parameter, executing a shell and launching manually still works); so we need to think in a way to detect child processes and wait for them, or at least add a flag to enable this feature. Disabling `--die-with-parent` is another option, but that would undermine security a bit and leave lingering wine processes all over the place. Maybe add a `--lead-process=NAME_EXE:TIMEOUT` to wait for another process inside the sandbox.
* Test under pure Wine 64-bit (see https://archlinux.org/news/transition-to-the-new-wow64-wine-and-wine-staging/ and https://gitlab.winehq.org/wine/wine/-/releases/wine-9.0#wow64)
* Implement bash autocompletion, should be able to autocomplete prefix and runner names based on the ones detected under Bottles. Also consider using [clap_complete](https://crates.io/crates/clap_complete).
* Add `integrate` sub-command to create integrations e.g., `.desktop` shortcut, entry on Heroic launcher.
* Native wayland support, see https://www.phoronix.com/news/Wine-9.22-Released and https://wiki.archlinux.org/title/Wine#Wayland. Also consider bringing back `--unshare-ipc` if using Wayland prevents the issue described in bwrap.rs#90.
* Add `kill` sub-command to terminate all processes in a sandbox, need to connect to existing bwrap container.
* When using the `integrate` sub-command to create a `.desktop` shortcut, extract executable icon and set it respectively. It can be done with a small windows executable calling a win32 API call or natively on Linux by using `wrestool`.
* Add NTSYNC support, see also https://www.phoronix.com/news/Linux-6.14-Char-Misc-NTSYNC.
* Add `--gpu` param (enum with default) to force dedicated GPU, see also:
  * https://wiki.archlinux.org/title/PRIME#Configure_applications_to_render_using_GPU
  * https://download.nvidia.com/XFree86/Linux-x86_64/435.17/README/primerenderoffload.html
  * https://wiki.manjaro.org/index.php/Configure_Graphics_Cards
  * https://wiki.archlinux.org/title/Hybrid_graphics
  * https://wiki.archlinux.org/title/PRIME#Note_about_Windows_games
* Detect dedicated GPU and enable `--gpu` param automatically

| Environment Variable      | Purpose                                                      | Typical Values                        | Affects                              | Notes                                                                                          |
|---------------------------|--------------------------------------------------------------|---------------------------------------|--------------------------------------|------------------------------------------------------------------------------------------------|
| DRI_PRIME                 | Selects which GPU to use for rendering (in Mesa/DRI stack)   | 0 (default GPU), 1 (dGPU)             | Which GPU handles rendering          | Used mostly on systems using the Mesa driver; 1 for discrete GPU rendering.                    |
| __NV_PRIME_RENDER_OFFLOAD | Enables NVIDIA's PRIME render offload mode                   | 1                                     | Activates NVIDIA render offload mode | Must be set to 1 to use NVIDIA GPU for rendering in hybrid setups.                             |
| __GLX_VENDOR_LIBRARY_NAME | Specifies which GLX vendor library to load (GLX client side) | nvidia, mesa                          | Determines which GLX implementation  | Should be nvidia for NVIDIA offload; mesa for default integrated GPU. Required for proper GLX. |
| __VK_LAYER_NV_optimus     | Ensures Vulkan applications use the correct GPU              | (empty), NVIDIA_only, non_NVIDIA_only | Vulkan applications                  | A value of NVIDIA_only causes to only report NVIDIA GPUs to the Vulkan application.            |
| DXVK_FILTER_DEVICE_NAME   | Set the GPU used by DXVK                                     | (empty), (device_name)                | Games ran by DXVK                    | Get the card name from vulkaninfo; DXVK uses substring match.                                  |

* Test with `DRI_PRIME=1 glxinfo | grep -E "OpenGL (vendor|renderer)"`, bear in mind that GPU may be powered-off on the first time, subsequent launches should be faster
* The `prime-run` command is just a script that sets the aforementioned variables: https://gitlab.archlinux.org/archlinux/packaging/packages/nvidia-prime/-/blob/main/prime-run?ref_type=heads

#### Packaging

* cURL install script.
* Create deb package. It should depend on Steam libraries (similarly to Arch's `steam-native-runtime`), see https://packages.ubuntu.com/search?keywords=steam&searchon=names&suite=noble&section=all.
* Make a reusable lib version (`Cargo.lock` needs to be ignored, see https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html).

#### Maybe

* Simple GUI delivered as Flatpak that builds the needed commands based on the selected options, and creates `.desktop` shortcuts.
* Investigate a way to use `--new-session` while allowing the user to read the output, without relying on seccomp, probably an easy fix could be to create an HTTP server where the output can be seen.
* Fork `steam-native-runtime` and remove Steam related stuff (i.e., keep dependencies only) and implement GitHub Actions for update checking and deployment to the AUR. This would prevent the `pacman.conf` workaround described in the FAQ.
* Create overlay filesystem on top of game directory in order to allow writing data without affecting the underlying files (could be used instead of `:rw`).
