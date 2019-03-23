<template>
    <v-container fluid>
      <v-layout row wrap justify-center>
        <v-flex xs12 sm8 md6>
          <v-text-field
            height="10%"
            solo
            v-model="searchTarget"
            append-icon="search"
            placeholder="Find Something.."
            @keyup.enter.native="addSearch"
            @click:append="addSearch"
          ></v-text-field>
        </v-flex>
        <v-flex xs12 v-if="isloading">
          <Loading/>
        </v-flex>
        <v-flex xs12>
          <StoreItems v-if="storeItems !== null" v-bind:storeItems="storeItems"/>
        </v-flex>
      </v-layout>
      <v-dialog v-model="regionDialog" scrollable max-width="200px">
      <template v-slot:activator="{ on }">
        <v-btn fab bottom right color="blue" dark fixed v-on="on">
          <v-icon>language</v-icon>
        </v-btn>
      </template>
      <v-card>
        <v-card-title>Select Your Region</v-card-title>
        <v-divider></v-divider>
        <v-card-text>
          <v-radio-group v-model="storeChange" column>
            <v-radio label="US" value="en:US"></v-radio>
            <v-radio label="UK" value="en:UK"></v-radio>
            <v-radio label="HongKong" value="en:HK"></v-radio>
          </v-radio-group>
        </v-card-text>
        <v-divider></v-divider>
        <v-card-actions>
          <v-btn color="blue darken-1" flat @click="regionDialog = false">Close</v-btn>
          <v-btn color="blue darken-1" flat @click="changeStoreSetting">Save</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
    </v-container>
    
</template>

<script>
import SearchBar from "@/components/SearchBar";
import StoreItems from "@/components/StoreItems";
import Loading from "@/components/Loading";

export default {
  name: "store",
  components: {
    SearchBar,
    Loading,
    StoreItems
  },
  data() {
    return {
      storeItems: null,
      isloading: false,
      searchTarget: "",

      storeChange: null,
      regionDialog: false,
      currentRegion: { language: "en", region: "US" }
    };
  },
  methods: {
    async addSearch() {
      try {
        if (this.searchTarget === "") throw "Wrong query request";
        this.storeItems = null;
        this.isloading = true;

        const response = await fetch(
          `${process.env.VUE_APP_PSNURL}store/${this.searchTarget}/${
            this.currentRegion.language
          }/${this.currentRegion.region}/21`
        );
        const items = await response.json();
        if (items.statusCode === 500) throw items.message;
        this.storeItems = {
          region: this.currentRegion.region,
          items: [...items]
        };
        this.isloading = false;
        this.searchTarget = "";
      } catch (err) {
        this.isloading = false;
        this.$emit("gotSnack", { error: err });
      }
    },
    changeStoreSetting() {
      const array = this.storeChange.split(":");
      this.currentRegion.language = array[0];
      this.currentRegion.region = array[1];
      this.regionDialog = false;
    }
  }
};
</script>
