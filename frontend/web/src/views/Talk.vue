<template>
    <v-container>
        <v-layout row wrap justify-center fluid>
            <v-speed-dial fixed bottom right fab v-if="profile">
                <template v-slot:activator>
                    <v-btn color="blue darken-2" dark fab to="/addpost" v-ripple>
                        <v-icon>create</v-icon>
                    </v-btn>
                </template>
            </v-speed-dial>
            <v-flex xs12 sm9 md7 lg6 xl5>
                <v-tabs v-model="active" slider-color="black" fixed-tabs>
                    <v-tab
                            v-for="n in categories.length"
                            :key="n"
                            ripple
                            light
                            @click="getCategory(n)"
                    >{{ categories[n - 1].name }}
                    </v-tab>
                        <v-tab-item v-for="n in categories.length" :key="n">
                            <template v-for="(topic, index) in topics">
                                <div class='thread_display' hover :key="index">
                                    <v-avatar class="thread_display__icon elevation-4"
                                              v-bind:size="$vuetify.breakpoint.smAndUp? '50' : '35'">
                                        <img src="https://upload.wikimedia.org/wikipedia/commons/e/e8/CandymyloveYasu.png">
                                    </v-avatar>
                                    <div style='width: 100%;' @click='show_topic(topic.id)' v-ripple>
                                        <div class='thread_display__header'>
				                        <span class='thread_display__name'>
                                            {{topic.id}}
					                        {{topic.title}}
				                        </span>
                                            <div class='thread_display__meta_bar'>
                                                <div>
                                                    From
                                                    <span class='thread_display__username font-weight-bold'
                                                          ref='username'>{{topic.user.username}}</span>
                                                    in
                                                    <span class='thread_display__category' ref='category'>{{topic.category_id}}</span>
                                                    &middot;
                                                    <span class='thread_display__date'><timeago
                                                            :datetime="topic.last_reply_time" :auto-update="60"
                                                            class="body-2 font-weight-thin">
                                                </timeago>
                                                </span>
                                                </div>
                                            </div>
                                        </div>
                                        <div class='thread_display__replies_bar'>
                                            <div class='thread_display__latest_reply' v-if="topic.reply_count >0">
                                                <span class='fa fa-reply fa-fw'></span>
                                                <span class='thread_display__latest_reply__text'>Latest reply by</span>
                                                <span class='thread_display__username'>"trest"</span>
                                                &middot;
                                                <span class='thread_display__date'>"test"</span>
                                            </div>
                                            <span style='cursor: default;' v-else>No replies</span>
                                            <div class='thread_display__replies' title='Replies to thread'
                                                 v-if="topic.reply_count >0">
                                                <span class='fa fa-comment-o fa-fw'></span>
                                                {{topic.reply_count }}
                                            </div>
                                        </div>
                                        <div class='thread_display__content'>
                                            <v-img contain max-height="300" position="start"
                                                   v-bind:src="topic.thumbnail"/>


                                        </div>
                                    </div>
                                </div>
                            </template>
                            <mugen-scroll :handler="loadMore" :should-handle="!isLoading" :handle-on-mount="false">
                                Loading....
                            </mugen-scroll>
                        </v-tab-item>
                </v-tabs>
            </v-flex>
        </v-layout>
    </v-container>
</template>


<script>
    import MugenScroll from 'vue-mugen-scroll';

    export default {
        name: "talk",
        props: ["profile"],
        components: {
            MugenScroll
        },
        data() {
            return {
                isLoading: false,
                selected: [1],
                categories: [],
                bottom: false,
                userDetailMenu: false,
                topics: [],
                page: 1,
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
            try {
                const get_cat = await fetch(`${process.env.VUE_APP_COMMURL}/categories/`);
                const categories = await get_cat.json();
                if (categories.error) throw categories.error;
                this.categories = categories;

                const get_pop = await fetch(`${process.env.VUE_APP_COMMURL}/categories/popular/1`);
                const popluar = await get_pop.json();
                if (popluar.error) throw popluar.error;
                this.topics = this.alter_topics(popluar);

            } catch (e) {
                this.$emit("gotSnack", {error: e})
            }
        },
        methods: {
            async getCategory(category_index) {
                this.page = 1;
                this.load_topics(category_index, this.page);
            },
            async loadMore() {
                this.isLoading = true;
                this.page = this.page + 1;
                console.log(this.selected);
                this.load_topics(this.selected, this.page);
                this.isLoading = false;
            },
            show_topic(topic_id) {
                setTimeout(() => {
                    this.$router.push({name: 'topic', params: {topic_id}})
                }, 200)

            },
            async load_topics(category_index, page) {
                try {
                    const response = await fetch(
                        `${process.env.VUE_APP_COMMURL}/categories/${category_index}/${page}`
                    );
                    const result = await response.json();
                    if (result.error) throw result.error;
                    this.topics = this.topics.concat(this.alter_topics(result));
                } catch (e) {
                    this.$emit("gotSnack", {error: e})
                }
            },
            alter_topics(response_json) {
                response_json.map(topic => {
                    if (topic.thumbnail !== "") {
                        topic.thumbnail = `${process.env.VUE_APP_COMMURL}/public/${topic.thumbnail}`
                    }
                    this.categories.forEach(category => {
                        if (category.id === topic.category_id) {
                            topic.category_id = category.name;
                        }
                    })
                });
                return response_json;
            },
        }
    };
</script>

<style lang="scss">
    @import '../assets/scss/variables.scss';

    .thread_display {
        background-color: #fff;
        border: thin solid $color__gray--darker;
        border-radius: 0.25rem;
        cursor: pointer;
        display: flex;
        margin-bottom: 1rem;
        padding: 0.75rem;
        position: relative;
        transition: background-color 0.1s, box-shadow 0.1s;

        &:hover {
            @extend .shadow_border--hover;
        }

        @at-root #{&}__icon {
            margin-right: 0.5rem;
        }

        @at-root #{&}__username,
        #{&}__category,
        #{&}__date {
            font-size: 1.15rem;
            color: $color--text__primary;
        }

        @at-root #{&}__header {
            display: column;
            justify-content: space-between;
        }
        @at-root #{&}__name {
            font-weight: 500;
            font-size: 1.25rem;
        }
        @at-root #{&}__meta_bar {
            display: flex;
            color: $color--gray__darkest;
            justify-content: space-between;
        }

        @at-root #{&}__replies_bar {
            display: flex;
            justify-content: space-between;
        }
        @at-root #{&}__latest_reply {
            color: $color--text__secondary;

            .fa {
                color: $color--text__primary;
                font-size: 0.75rem;
            }
        }
        @at-root #{&}__replies {
            width: 4rem;
            text-align: right;
        }

        @at-root #{&}__content {
            margin-top: 0.5rem;
            word-break: break-all;
        }
    }

    @media (max-width: 420px) {
        .thread_display {
            @at-root #{&}__header {
                flex-direction: column;
            }
            @at-root #{&}__meta_bar {
                font-size: 0.9rem;
                margin-bottom: 0.25rem;
            }

            @at-root #{&}__content {
                margin-top: 0.5rem;
                margin-left: -3rem;
                word-break: break-all;
            }

            @at-root #{&}__replies_bar {
                position: relative;
                left: -3rem;
                width: calc(100% + 3rem);
            }

            @at-root #{&}__latest_reply {
                .fa {
                    margin-right: 0.25rem;
                }

                @at-root #{&}__text {
                    display: none;
                }
            }
        }
    }

</style>

