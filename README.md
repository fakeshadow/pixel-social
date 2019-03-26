### **A Simple Community App backend**

#### Requirement:
`Rustup 1.33 and above`<br>
`PostgreSQL 10 and above`<br>
`Flutter 1.2 and above`<br>
`Vue cli 3.0 and above`

#### Setup:
`Backend:`<br>
1. install `diesel cli` with `cargo install diesel_cli --no-default-features --features postgres`
2. `diesel migration run` to init database
3. `cargo build --release`<br>
4. Make change to `.env` file to match your environment.<br>
*. make changes to cors setting if you encounter the issue.
5. run the compiled `pixel_rust` bin file to start the server
    
`Web Frontend:`<br>
1. `yarn install`
2. `yarn build`
3. setup up http server and change the `.env` file to match your backend server.


