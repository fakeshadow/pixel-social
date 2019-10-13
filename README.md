### **A Simple Community App backend**

#### Requirement:
`rust 1.39.0-beta.5 (fa5c2f3e5 2019-10-02)`<br>
`PostgreSQL 10 and above`<br>
`Redis 5.0.4 and above`<br>
`Flutter 1.5 and above with flutter_web preview`<br>

#### Setup:
`Backend:`<br>
1. Make change to `.env` file to match your environment.<br>
*. make changes to cors setting if you encounter the issue.
2. `cargo build --release`<br>
3. run the compiled `pixel_social` in target/release folder to start the server<br>
*. run `pixel_social build` to generate dummy tables.<br>
*. run `pixel_social drop` to drop all dummy tables.
    
`Web Frontend:`<br>
1. `pub get` to get dependencies
2. `webdev serve` to test locally

`Mobile Frontend:`<br>
1. `flutter run` to test on simulator or `flutter run --profile` for physic debug.
