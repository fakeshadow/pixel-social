<template>
    <v-container>
        <v-layout row wrap justify-center>
            <v-flex xs12 lg6 xl5>
                <v-container>
                    <v-card light max-width="100%" hover>
                        <v-card-title>
                            <div>
                                <v-avatar size="60">
                                    <img src="https://upload.wikimedia.org/wikipedia/commons/e/e8/CandymyloveYasu.png">
                                </v-avatar>
                            </div>
                            <div class="headline ml-3">
                                {{topic.user.username}}<br>
                                <timeago :datetime="topic.last_reply_time" :auto-update="60"
                                         class="subheading font-weight-thin"></timeago>
                            </div>
                        </v-card-title>
                        <v-card-text>
                            <v-textarea v-html="topic.body"
                                        v-bind:class="$vuetify.breakpoint.smAndUp? 'text' : 'text-xs'"></v-textarea>
                        </v-card-text>
                    </v-card>
                </v-container>
                <div v-if="posts !=[]">
                    <template v-for="(post, index) in posts">
                        <v-container :key="index" v-if="posts !== null">
                            <v-card light max-width="100%" hover>
                                <v-card-title>
                                    <div>
                                        <v-avatar size="60">
                                            <img src="https://upload.wikimedia.org/wikipedia/commons/e/e8/CandymyloveYasu.png">
                                        </v-avatar>
                                    </div>
                                    <div class="headline ml-3">
                                        {{post.user.username}}<br>
                                        <timeago :datetime="post.last_reply_time" :auto-update="60"
                                                 class="subheading font-weight-thin"></timeago>
                                    </div>
                                </v-card-title>
                                <v-card-text>
                                    <v-textarea v-html="post.post_content"
                                                v-bind:class="$vuetify.breakpoint.smAndUp? 'text' : 'text-xs'"></v-textarea>
                                </v-card-text>
                            </v-card>
                        </v-container>
                    </template>
                </div>

            </v-flex>
        </v-layout>
    </v-container>
</template>


<script>
    import Loading from "@/components/Loading";

    export default {
        name: "topic",
        components: {
            Loading
        },
        data() {
            return {
                categories: ["General", "Share", "Other"],
                isLoading: false,
                topic_id: null,
                topic: [],
                posts: [],
                first_page: 1,
            };
        },
        async created() {
            try {
                if (!this.$route.params.topic_id) {
                    this.topic_id = localStorage.topic_id;
                } else {
                    this.topic_id = this.$route.params.topic_id;
                }
                this.isLoading = true;
                const response = await fetch(`${process.env.VUE_APP_COMMURL}/topic/${this.topic_id}/${this.first_page}`);
                const result = await response.json();
                if (result.error) throw result.error;
                this.posts = result.posts;
                this.topic = result.topic;
                localStorage.topic_id = this.topic_id;
            } catch (e) {
                this.$emit("gotSnack", {error: e})
            }
            this.isLoading = false;
        },
        methods: {}
    };
</script>

<style>
    .text {
        margin-left: 5.5rem;
        margin-top: -1.5rem;
    }
    .text-xs {
        margin-top: -1.5rem;
    }

</style>