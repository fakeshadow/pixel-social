<template>
  <v-container fluid>
    <v-layout row wrap justify-center>
      <v-flex xs12 sm8 md6>
        <v-text-field
          height="10%"
          solo
          v-model="searchTarget"
          append-icon="search"
          placeholder="Find Someone.."
          @keyup.enter.native="addSearch"
          @click:append="addSearch"
        ></v-text-field>
      </v-flex>
      <v-flex xs12 v-if="isloading">
        <Loading/>
      </v-flex>
      <v-flex xs12 lg8>
        <Profile v-if="profile !== null" v-bind:profile="profile"/>
      </v-flex>
    </v-layout>
  </v-container>
</template>

<script>
import Profile from "@/components/Profile";
import Loading from "@/components/Loading";

export default {
  name: "people",
  components: {
    Profile,
    Loading
  },
  data() {
    return {
      profile: null,
      isloading: false,
      searchTarget: ""
    };
  },
  methods: {
    async addSearch() {
      try {
        if (this.searchTarget === "") throw "Wrong query request";
        this.profile = null;
        this.isloading = true;
        const response = await fetch(process.env.VUE_APP_PSNURL, {
          method: "post",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify({
            onlineId: this.searchTarget
          })
        });
        const _profile = await response.json();

        if (_profile.error) throw _profile.message;
        this.profile = _profile;
        this.isloading = false;
        this.searchTarget = "";
      } catch (err) {
        this.$emit("gotSnack", { error: err });
        this.isloading = false;
      }
    }
  }
};
</script>
