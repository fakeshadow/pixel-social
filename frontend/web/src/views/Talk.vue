<template>
    <v-container fluid>
        <v-layout row wrap justify-center fluid>
            <v-speed-dial fixed bottom right fab>
                <template v-slot:activator>
                    <v-btn color="blue darken-2" dark fab to="/addpost" v-ripple>
                        <v-icon>create</v-icon>
                    </v-btn>
                </template>
            </v-speed-dial>
            <v-flex xs12 sm10 md9 lg8 xl5>
                <v-tabs v-model="active" slider-color="black" fixed-tabs>
                    <v-tab
                            v-for="n in categories.length"
                            :key="n"
                            ripple
                            light
                            @click="getCategory(n)"
                    >{{ categories[n - 1] }}
                    </v-tab>
                    <v-tab-item v-for="n in categories.length" :key="n">
                        <template v-for="(d, index) in data">
                            <v-container :key="index" pa-1>
                                <v-card light max-width="100%" hover>
                                    <v-layout row wrap>
                                        <v-flex xs8>
                                            <v-list>
                                                <v-list-tile>
                                                    <v-menu
                                                            :close-on-content-click="true"
                                                            :nudge-width="300"
                                                            transition="slide-x-transition"
                                                            bottom
                                                            left
                                                            offset-x
                                                    >
                                                        <template v-slot:activator="{ on }">
                                                            <v-list-tile-avatar v-ripple v-on="on" :size="50"
                                                                                class="pt-3">
                                                                <img
                                                                        src="https://upload.wikimedia.org/wikipedia/commons/e/e8/CandymyloveYasu.png"
                                                                >
                                                            </v-list-tile-avatar>
                                                        </template>
                                                        <v-card>
                                                            <v-img :src="cards[0].src" height="200px">
                                                                <v-container fill-height fluid pa-2>
                                                                    <v-layout fill-height>
                                                                        <v-flex xs12 align-end flexbox>
                                                                            <span class="headline white--text"
                                                                                  v-text="cards[0].title"></span>
                                                                        </v-flex>
                                                                        <v-flex xs12 align-end flexbox>
                                                                            <v-btn icon>
                                                                                <v-icon>favorite</v-icon>
                                                                            </v-btn>
                                                                        </v-flex>
                                                                    </v-layout>
                                                                </v-container>
                                                            </v-img>
                                                        </v-card>
                                                    </v-menu>

                                                    <v-list-tile-content class="pl-3 pt-1">
                                                        <v-list-tile-title class="subheading font-weight-black">
                                                            {{d.user.username}}
                                                        </v-list-tile-title>
                                                        <v-list-tile-sub-title class="subheading font-weight-thin">
                                                            <timeago :datetime="d.last_reply_time"
                                                                     :auto-update="60"></timeago>
                                                        </v-list-tile-sub-title>
                                                    </v-list-tile-content>
                                                </v-list-tile>
                                            </v-list>
                                        </v-flex>

                                        <v-flex xs4></v-flex>
                                        <v-flex xs12>
                                            <v-list three-line>
                                                <v-list-tile>
                                                    <v-list-tile-avatar v-if="$vuetify.breakpoint.smAndUp" :size="50">
                                                    </v-list-tile-avatar>
                                                    <v-list-tile-content class="test">
                                                        <v-list-tile-sub-title v-ripple
                                                                               class="subheading font-weight-black pl-3 margin"
                                                                               @click="show_topic(d.id)">
                                                            {{d.title}}
                                                        </v-list-tile-sub-title>
                                                    </v-list-tile-content>
                                                </v-list-tile>
                                            </v-list>
                                        </v-flex>
                                    </v-layout>

                                </v-card>
                            </v-container>
                        </template>
                    </v-tab-item>
                </v-tabs>
            </v-flex>
        </v-layout>
    </v-container>
</template>


<script>
    import Loading from "@/components/Loading";

    export default {
        name: "talk",
        components: {
            Loading
        },
        data() {
            return {
                categories: ["General", "Share", "Other"],
                userDetailMenu: false,
                isLoading: false,
                data: [],
                first_page: 1,
                active: 0,
                cards: [
                    {
                        title: "Pre-fab homes",
                        src: "https://cdn.vuetifyjs.com/images/cards/house.jpg",
                        flex: 12
                    },
                    {
                        title: "Favorite road trips",
                        src: "https://cdn.vuetifyjs.com/images/cards/road.jpg",
                        flex: 6
                    },
                    {
                        title: "Best airlines",
                        src: "https://cdn.vuetifyjs.com/images/cards/plane.jpg",
                        flex: 6
                    }
                ]
            };
        },
        async mounted() {
            this.isLoading = true;
            const response = await fetch(`${process.env.VUE_APP_COMMURL}/categories/popular/1`);
            const json = await response.json();
            this.data = json;
            this.isLoading = false;
        },
        methods: {
            async getCategory(category_index) {
                try {
                    this.isLoading = true;
                    const response = await fetch(
                        `${process.env.VUE_APP_COMMURL}/categories/${category_index}/${this.first_page}`
                    );
                    const result = await response.json();
                    if (result.error) throw result.error;
                    this.data = result;
                    this.isLoading = false;
                } catch (e) {
                    this.$emit("gotSnack", {error: e})
                }
                this.isLoading = false;
            },
            show_topic(topic_id) {
                setTimeout(() => {
                    this.$router.push({name: 'topic', params: {topic_id}})
                }, 200)

            }
        }
    };
</script>

<style>
    .margin {
        margin-top: -1.2em;
        margin-left: -0.2em;
    }
</style>

