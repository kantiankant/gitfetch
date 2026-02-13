
Version: 0.18.x Beta

1.0: An introduction 

This is gitfetch, a security-focused, minimal wrapper for git that acts as a pseudo-package manager that serves the needs of users that have a desire for security, minimalism, and functionality. Gitfetch is licensed under the GNU Public License Version 3 (See the LICENSE file for more information) and was written entirely in rust, excluding the install and update shell scripts.

1.1: Installation.

First and foremost, clone the repository (via git clone https://github.com/kantiankant/gitfetch.git) to clone the repository to your current working directory. Now, change your working directory to the cloned gitfetch repository's directory (cd gitfetch/), then run the install script, and it will compile and install the binary to your system for you.

1.2: Usage

Usage: gitfetch [COMMAND]

Commands:
  clone, -c       Clone a repository with ACTUAL security (hooks disabled, network isolated) [aliases: clone]
  list, -l        List all the repos you've installed with this nonsense [aliases: list]
  search, -s      Search for repositories by name [aliases: search]
  easter-egg, -e  Print something utterly pointless [aliases: easter-egg]
  completions     Generate shell completion scripts
  checksum        Calculate checksums for a cloned repository [aliases: checksum]
  verify          Verify repository integrity against saved checksums [aliases: verify]
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

TRUST MODES (for clone command):
  paranoid  - Maximum security: verify everything, prompt for all decisions, isolate network
  normal    - Default: balanced security with reasonable prompts and standard checks
  yolo      - Minimal security: trust the source, minimal prompts (use with caution)
Usage: gitfetch clone <repo> --trust-mode <mode>

1.3: updating gitfetch

Change your current working directory to gitfetch/, and simply run the provided update script (update.sh), and it will fetch the latest updates and compile them for you. NOTE: run cargo clean in the working directory beforehand to ensure that it re-compiles gitfetch from source.

this was inspired by a friend's project, go check his stuff out at: nyancqt/ghpm

Dissect
