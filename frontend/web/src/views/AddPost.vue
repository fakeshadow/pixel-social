<template>
    <v-container>
        <v-layout justify-center>
            <v-flex xs12 lg6>
                <v-form>
                    <v-layout row wrap>
                        <v-flex xs2>
                            <v-select
                                    :items="category_names"
                                    v-model="topic_data.category_id"
                                    label="Category"
                                    outline
                                    :height="30"
                            ></v-select>
                        </v-flex>
                        <v-flex xs10>
                            <v-text-field
                                    v-model="topic_data.title"
                                    outline
                                    single-line
                                    placeholder="Please input title here"
                                    clear-icon="mdi-close-circle"
                                    clearable
                                    type="text"
                            ></v-text-field>
                        </v-flex>
                        <v-flex xs12 text-xs-center>
                            <v-btn @click="removeImage" color="primary" v-if="topic_data.thumbnail">Remove Thumbnail</v-btn>
                            <v-menu bottom origin="center center" transition="scale-transition" v-if="!topic_data.thumbnail">
                                <template v-slot:activator="{ on }">
                                    <v-btn color="primary" dark v-on="on"> Add a thumbnail?
                                    </v-btn>
                                </template>
                                <v-list>
                                    <v-list-tile>
                                        <input type="file" @change="onFileChange">
                                    </v-list-tile>
                                </v-list>
                            </v-menu>
                        </v-flex>
                        <v-flex xs12 text-xs-center v-if="topic_data.thumbnail" justify-center>
                            <img :src="topic_data.thumbnail" width="300">
                        </v-flex>

                        <v-flex xs12 class="pt-4">
                                <ckeditor :editor="editor" v-model="topic_data.body" :config="editorConfig" class="ck-editor__editable">
                                </ckeditor>
                        </v-flex>
                        <v-flex xs12 text-xs-center>
                            <v-btn v-ripple color="primary" @click="addTopic">
                                Submit
                            </v-btn>
                        </v-flex>

                    </v-layout>
                </v-form>
            </v-flex>
        </v-layout>
    </v-container>
</template>

<script>
    import ClassicEditor from "@ckeditor/ckeditor5-build-classic";

    export default {
        name: "addpost",
        data() {
            return {
                categories: null,
                category_names: [],
                action_menu_items: [
                    {title: "Add thumbnail"},
                    {title: "Add tags"},
                ],
                show_add_picture: false,
                show_add_tags: false,
                show_err: false,
                editor: ClassicEditor,
                editorData: "",
                editorConfig: {
                    placeholder: "Have fun posting"
                },
                topic_data: {
                    category_id: 1,
                    title: "",
                    body: "",
                    thumbnail: "",
                }
            };
        },
        async mounted() {
            const response = await fetch(
                `${process.env.VUE_APP_COMMURL}/categories/`
            );
            const json = await response.json();
            this.categories = json;
            json.forEach(category => this.category_names.push(category.name));
        },
        beforeDestroy() {
        },
        methods: {
            async addTopic() {
                try {
                    if (this.topic_data.title.length < 8) throw new Error("Title too short.");
                    if (this.topic_data.body.length < 8) throw new Error("Content too short.");

                    this.categories.forEach(category => {
                        if (category.name === this.topic_data.category_id) {
                            return this.topic_data.category_id = category.id;
                        }
                    });

                    const response = await fetch(`${process.env.VUE_APP_COMMURL}/topic/`, {
                        method: "post",
                        body: JSON.stringify(this.topic_data),
                        headers: {
                            "Content-Type": "application/json",
                            "Authorization": `Bearer ${localStorage.jwt}`
                        }
                    });
                    const json = await response.json();

                    if (json.error) throw json.error;

                    this.topic_data = {
                        category_id: 1,
                        title: "",
                        body: "",
                        thumbnail: "",
                    };
                    this.$emit("gotSnack", {success: "Topic delivered safely"});
                } catch (e) {
                    this.$emit("gotSnack", {error: e});
                }
            },
            menu_action(index) {
                if (index === 0) {
                    this.show_add_picture = true;
                } else if (index === 1) {
                    this.show_add_tags = true;
                }
            },
            onFileChange(e) {
                this.show_err = false;
                const files = e.target.files || e.dataTransfer.files;
                if (!files.length) return;
                if (files[0].size >= 999999) {
                    return this.$emit("gotSnack", {
                        error: "Image file too big. Please reduce the size to less than 1mb"
                    });
                }
                this.createImage(files[0]);
            },
            createImage(file) {
                const reader = new FileReader();

                reader.onload = e => {
                    this.topic_data.thumbnail = e.target.result;
                };
                reader.readAsDataURL(file);
            },
            removeImage() {
                this.topic_data.thumbnail = "";
            }
        }
    };
</script>

<style>
    .ck-editor__editable {
        min-height: 300px;
    }

</style>

