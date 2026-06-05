build:
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C lto -C strip=symbols -C embed-bitcode" cross build --release --target aarch64-unknown-linux-gnu
    cd ui-svelte && npm run build
    ssh -t pi@pi.local "sudo systemctl stop remote-yt.service"
    scp target/aarch64-unknown-linux-gnu/release/remote-yt pi@pi.local:/home/pi/remote-yt
    ssh pi@pi.local "mkdir -p /home/pi/remote-yt/ui-svelte/build"
    scp -r ui-svelte/build/* pi@pi.local:/home/pi/remote-yt/ui-svelte/build/
    ssh -t pi@pi.local "sudo systemctl start remote-yt.service"
