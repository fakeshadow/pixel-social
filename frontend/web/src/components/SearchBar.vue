<template>
  <v-layout row justify-center>
    <v-flex xs4 sm4 md3 lg3 xl3>
      <v-select solo label="Slect" v-bind:items="types" v-model="type"></v-select>
    </v-flex>
    <v-flex xs8 sm8 md9 lg9 xl9>
      <v-text-field
        height="10%"
        v-model="target"
        solo
        append-icon="search"
        placeholder="Find Something.."
        @click:append="addSearch"
      ></v-text-field>
    </v-flex>
  </v-layout>
</template>

<script lang="ts">
export default {
  name: "SearchBar",
  data() {
    return {
      type: null,
      types: ["Store", "People", "Deals"],
      target: ""
    };
  },
  methods: {
    addSearch(e) {
      e.preventDefault();
      if (this.target !== "" && this.type !== null) {
        const newSearch = {
          type: this.type,
          target: this.target,
          isloading: true
        };
        this.$emit("addSearch", newSearch);
        this.target = "";
      } else this.$emit("addSearch", "error");
    }
  }
};
</script>

<style scoped>
form {
  display: flex;
}

select {
  border-radius: 50%;
  flex: 2;
}

input[type="text"] {
  flex: 8;
  padding: 5px;
}

input[type="submit"] {
  flex: 2;
}
</style>

