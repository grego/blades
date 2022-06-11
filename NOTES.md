# NOTES

## Crosscompile from macos to Linux

Use lima https://github.com/lima-vm/lima

     limactl start --name ubuntu-x84_64 ./bin/lima/ubuntu-intel.yml

     limactl shell ubuntu-x84_64

     sudo apt install build-essential

     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    # default host triple is x86_64-unknown-linux-gnu
    source $HOME/.cargo/env

    cargo build --release