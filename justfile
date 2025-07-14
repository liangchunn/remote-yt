build:
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C lto -C strip=symbols -C embed-bitcode" cross build --release --target aarch64-unknown-linux-gnu
    cd ui && npm run build
    ssh -t pi@192.168.0.17 "sudo systemctl stop remote-yt.service"
    scp target/aarch64-unknown-linux-gnu/release/remote-yt pi@192.168.0.17:/home/pi/remote-yt/target/release
    scp -r ui/dist pi@192.168.0.17:/home/pi/remote-yt/ui
    ssh -t pi@192.168.0.17 "sudo systemctl start remote-yt.service"