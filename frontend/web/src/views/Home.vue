<template>
  <v-container justify-center text-xs-center>
    <v-layout column class="welcome">
      <v-card flat color="#ffffff00" width="60%">
        <v-card-title class="justify-center">
          <h1>Welcome to PixelShare</h1>
        </v-card-title>
        <v-divider></v-divider>
        <v-card-text>
          <br>
          <h3>You can access PSN API by starting a search or registering an account for more functions.</h3>
          <h5>* This site is for example use and no user info is collected.</h5>
          <br>
        </v-card-text>
        <v-card-actions class="justify-center">
          <v-btn color="blue-grey" class="white--text" @click="showDialog = true">
            <v-icon left dark>cloud_upload</v-icon>Check out source code
          </v-btn>
        </v-card-actions>
      </v-card>
      <v-dialog v-model="showDialog" max-width="320">
        <v-card justify-center text-xs-center>
          <v-card-title class="headline">You are about to open following link:</v-card-title>

          <v-card-text>https://github.com/fakeshadow/pixel-psn</v-card-text>

          <v-spacer></v-spacer>
          <v-card-actions class="justify-center">
            <v-btn @click="showDialog = false" href="https://github.com/fakeshadow/pixel-psn">OK</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </v-layout>
  </v-container>
</template>

<script>

export default {
  name: "home",
  data() {
    return {
      showDialog: false
    };
  },
  methods: {}
};
</script>



<style scoped>
.welcome {
  margin-top: 10%;
  justify-content: center;
  align-items: center;
}
</style>
