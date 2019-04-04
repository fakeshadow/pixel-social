<template>
    <v-dialog v-model="loginDialog" persistent max-width="310px">
        <template v-slot:activator="{ on }">
            <v-btn icon large v-on="on">
                <v-avatar size="32px" tile>
                    <img src="../assets/playstation-brands.svg">
                </v-avatar>
            </v-btn>
        </template>

        <v-card>
            <v-layout column>
                <v-card-title class="justify-center">
                    <span class="headline">Authentication Page</span>
                </v-card-title>
                <v-card-text>
                    <v-container grid-list-md>
                        <v-layout wrap>
                            <v-flex xs12>
                                <v-text-field v-model="username" label="UserName" required></v-text-field>
                            </v-flex>
                            <v-flex xs12>
                                <v-text-field v-model="password" label="Password" required></v-text-field>
                            </v-flex>
                            <v-flex xs12 text-xs-center>
                                <v-btn
                                        v-if="isRegister === false"
                                        color="blue darken-1"
                                        flat
                                        @click="isRegister = true"
                                >First time here?
                                </v-btn>
                            </v-flex>
                            <v-flex xs12>
                                <v-text-field v-if="isRegister === true" label="Email Address"
                                              v-model="email"></v-text-field>
                            </v-flex>
                        </v-layout>
                    </v-container>
                </v-card-text>
                <v-card-actions justify-center text-xs-center>
                    <v-spacer></v-spacer>
                    <v-btn
                            v-if="isRegister===false"
                            :loading="isLoading"
                            :disabled="isLoading"
                            @click="login"
                            Raised
                            color="blue darken-1"
                    >Login
                    </v-btn>
                    <v-btn
                            v-if="isRegister===true"
                            :loading="isLoading"
                            :disabled="isLoading"
                            @click="register"
                            Raised
                            color="blue darken-1"
                    >Register
                    </v-btn>
                    <v-btn color="blue darken-1" flat @click="closeLogin">Close</v-btn>
                </v-card-actions>
            </v-layout>
        </v-card>
    </v-dialog>
</template>

<script>
    export default {
        name: "AuthDialog",
        data() {
            return {
                isRegister: false,
                isLoading: false,
                loginDialog: false,
                username: "",
                password: "",
                email: ""
            };
        },
        methods: {
            closeLogin() {
                this.isRegister = false;
                this.loginDialog = false;
                this.email = "";
                this.username = "";
                this.password = "";
                this.isLoading = false;
            },
            async login() {
                try {
                    this.isLoading = true;
                    await this.get_login();
                } catch (e) {
                    this.isLoading = false;
                    this.$emit("gotSnack", {error: e});
                }
            },
            async register() {
                try {
                    this.isLoading = true;

                    const response = await fetch(`${process.env.VUE_APP_COMMURL}/user/register`, {
                        method: "post",
                        body: JSON.stringify({
                            username: this.username,
                            password: this.password,
                            email: this.email
                        }),
                        headers: {"Content-Type": "application/json"}
                    });
                    const result = await response.json();

                    if (result.error) {
                        throw result.error;
                    } else {
                        await this.get_login();
                    }
                    this.closeLogin();
                } catch (e) {
                    this.isLoading = false;
                    this.$emit("gotSnack", {error: e});
                }
            },
            async get_login() {
                const response = await fetch(`${process.env.VUE_APP_COMMURL}/user/login`, {
                    method: "post",
                    body: JSON.stringify({
                        username: this.username,
                        password: this.password
                    }),
                    headers: {"Content-Type": "application/json"}
                });
                const result = await response.json();

                if (result.error) {
                    throw result.error;
                } else {
                    this.$emit("gotLogin", result);
                    this.$emit("gotSnack", {success: "Login Success"});
                }
                this.closeLogin();
            }
        }
    };
</script>

<style scoped>
</style>
