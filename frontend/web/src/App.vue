<template>
    <v-app>
        <v-navigation-drawer v-model="drawer" :clipped="true" fixed app disable-resize-watcher>
            <v-list>
                <v-list-tile to="/">
                    <v-list-tile-avatar>
                        <v-icon>account_box</v-icon>
                    </v-list-tile-avatar>
                    <v-list-tile-title>Home</v-list-tile-title>
                </v-list-tile>
                <v-list-tile to="/store">
                    <v-list-tile-avatar>
                        <v-icon>settings</v-icon>
                    </v-list-tile-avatar>
                    <v-list-tile-title>Store</v-list-tile-title>
                </v-list-tile>
                <v-list-tile to="/people">
                    <v-list-tile-avatar>
                        <v-icon>exit_to_app</v-icon>
                    </v-list-tile-avatar>
                    <v-list-tile-title>People</v-list-tile-title>
                </v-list-tile>
                <v-list-tile to="/talk">
                    <v-list-tile-avatar>
                        <v-icon>exit_to_app</v-icon>
                    </v-list-tile-avatar>
                    <v-list-tile-title>Talk</v-list-tile-title>
                </v-list-tile>
            </v-list>
        </v-navigation-drawer>
        <v-toolbar
                :clipped-left="$vuetify.breakpoint.lgAndUp"
                color="blue"
                dark
                app
                fixed
                scroll-off-screen
        >
            <v-toolbar-title style="width: 300px" class="ml-0 pl-3">
                <v-toolbar-side-icon @click.stop="drawer = !drawer" class="hidden-md-and-up"></v-toolbar-side-icon>
                <span class="hidden-sm-and-down">
          <v-btn to="/" flat ripple class="text-none">
            <h1>PixelShare</h1>
          </v-btn>
        </span>
            </v-toolbar-title>
            <v-toolbar-items class="hidden-sm-and-down">
                <v-btn to="/store" flat>Store</v-btn>
                <v-btn to="/people" flat>People</v-btn>
                <v-btn to="/talk" flat>Talk</v-btn>
                <v-btn flat>About</v-btn>
            </v-toolbar-items>
            <v-spacer></v-spacer>

            <AuthDialog v-on:gotLogin="gotLogin" v-on:gotSnack="gotSnack" v-if="jwt === null"/>
            <UserMenu
                    v-else
                    v-on:gotLogout="gotLogout"
                    v-on:gotSnack="gotSnack"
                    v-bind:profile="profile"
            />
        </v-toolbar>
        <v-snackbar v-model="showSnack" :timeout="5000" top class="text-xs-center">
            {{ this.snackMessage }}
            <v-btn color="pink" flat @click="showSnack = false">Close</v-btn>
        </v-snackbar>
        <v-content>
            <router-view v-on:gotSnack="gotSnack"/>
        </v-content>
    </v-app>
</template>

<script>
    import AuthDialog from "@/components/AuthDialog";
    import UserMenu from "@/components/UserMenu";

    export default {
        name: "app",
        components: {
            AuthDialog,
            UserMenu
        },
        data() {
            return {
                drawer: false,
                jwt: null,
                profile: null,

                showSnack: false,
                snackMessage: null
            };
        },
        mounted() {
            if (localStorage.jwt && localStorage.profile) {
                this.jwt = localStorage.jwt;
                this.profile = JSON.parse(localStorage.profile);
            }
        },
        methods: {
            gotLogin(data) {
                const {token, user_data} = data;
                this.jwt = token;
                this.profile = user_data;
                localStorage.jwt = token;
                localStorage.profile = JSON.stringify(user_data);
            },
            gotLogout(boolean) {
                if (boolean) {
                    localStorage.removeItem("jwt");
                    localStorage.removeItem("username");
                    localStorage.removeItem("npId");
                    localStorage.removeItem("uid");
                    this.jwt = null;
                    this.profile = null;
                }
            },
            gotSnack(snack) {
                if (snack.error) {
                    this.snackMessage = snack.error;
                } else if (snack.success) {
                    this.snackMessage = snack.success;
                }
                this.showSnack = true;
            }
        }
    };
</script>

<style scoped>
</style>