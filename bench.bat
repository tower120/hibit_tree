@echo off
setlocal
REM set RUSTFLAGS=-C target-feature=+popcnt,+bmi1
set RUSTFLAGS=-C target-cpu=native
cargo bench --bench %1
endlocal