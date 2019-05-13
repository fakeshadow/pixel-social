### **A Simple Community App backend**

#### Requirement:
`Rustup 1.33 and above`<br>
`PostgreSQL 10 and above`<br>
`Redis 5.0.4 and above`<br>
`Flutter 1.5 and above with flutter_web preview`<br>

#### Setup:
`Backend:`<br>
1. install `diesel cli` with `cargo install diesel_cli --no-default-features --features postgres`<br>
*. go to postgreSQL install folder and run pg_env.bat if you encounter libpg.dll error when installing diesel cli
2. Make change to `.env` file to match your environment.<br>
*. make changes to cors setting if you encounter the issue.
3. `diesel setup` and `diesel migration run` to init database
4. `cargo build --release`<br>
5. run the compiled `pixel_rs` bin file to start the server
    
`Web Frontend:`<br>
1. `pub get` to get dependencies
2. `webdev serve` to test locally

`Mobile Frontend:`<br>
1. `flutter create .` to generate dep files.
2. `flutter run` to test on simulator or `flutter run --profile` for physic debug.
