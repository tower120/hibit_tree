@echo off
setlocal
set RUSTFLAGS=-C target-feature=+popcnt,+bmi1,+bmi2
REM set RUSTFLAGS=-C target-cpu=native
cargo bench --bench %1
endlocal