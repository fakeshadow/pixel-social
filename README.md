### **A Simple Community App backend**

#### Requirement:
`Rustup 1.33 and above`<br>
`PostgreSQL 10 and above`<br>
`Redis 5.0.4 and above`<br>
`Flutter 1.5 and above with flutter_web preview`<br>

#### Setup:
`Backend:`<br>
1. Make change to `.env` file to match your environment.<br>
*. make changes to cors setting if you encounter the issue.
2. `cargo build --release`<br>
3. run the compiled `pixel_rs` in target/release folder to start the server<br>
*. run `pixel_rs build` to generate dummy tables.<br>
*. run `pixel_rs drop` to drop all tables.
    
`Web Frontend:`<br>
1. `pub get` to get dependencies
2. `webdev serve` to test locally

`Mobile Frontend:`<br>
1. `flutter run` to test on simulator or `flutter run --profile` for physic debug.
