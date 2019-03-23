<template>
  <v-card dark>
    <v-card-text>
      <v-layout row justify-center text-xs-center>
        <v-flex xs4>
          <v-progress-circular :rotate="90" :size="60" :value="value" color="yellow">LV{{ level }}</v-progress-circular>
        </v-flex>
        <v-flex xs4>
          <v-progress-circular :rotate="90" :size="60" :value="value" color="blue">Rep{{ level }}</v-progress-circular>
        </v-flex>
        <v-flex xs4>
          <v-progress-circular :rotate="90" :size="60" :value="value" color="red">Rank{{ level }}</v-progress-circular>
        </v-flex>
      </v-layout>
    </v-card-text>
    <v-card-title>
      <v-layout column justify-center text-xs-center>
        <v-flex xs12>
          <v-avatar :size="150">
            <img v-bind:src="profile.avatarUrl" contain>
          </v-avatar>
        </v-flex>
        <v-flex xs12>
          <h1>{{profile.onlineId}}</h1>
          <h4>{{profile.aboutMe}}</h4>
        </v-flex>
      </v-layout>
    </v-card-title>

    <v-card-text>
      <v-layout row wrap text-xs-center>
        <v-flex xs3>
          <img src="@/assets/plat.gif" height="25">
          <span class="subheading mr-1">{{profile.trophySummary.earnedTrophies.platinum}}</span>
        </v-flex>
        <v-flex xs3>
          <img src="@/assets/gold.gif" height="25">
          <span class="subheading mr-1">{{profile.trophySummary.earnedTrophies.gold}}</span>
        </v-flex>
        <v-flex xs3>
          <img src="@/assets/silv.gif" height="25">
          <span class="subheading mr-1">{{profile.trophySummary.earnedTrophies.silver}}</span>
        </v-flex>
        <v-flex xs3>
          <img src="@/assets/bron.gif" height="25">
          <span class="subheading mr-1">{{profile.trophySummary.earnedTrophies.bronze}}</span>
        </v-flex>
      </v-layout>
    </v-card-text>
    <v-divider></v-divider>
    <v-list three-line v-if="profile.trophyList">
      <template v-for="(game,index) in profile.trophyList">
        <v-subheader
          v-if="game.npCommunicationId"
          :key="game.npCommunicationId"
        >{{ game.npCommunicationId }}</v-subheader>
        <v-list-tile :key="index">
          <v-list-tile-avatar>
            <img :src="profile.avatarUrl">
          </v-list-tile-avatar>

          <v-list-tile-content>
            <v-list-tile-title v-html="game.progress"></v-list-tile-title>
            <v-list-tile-sub-title v-html="game.lastUpdateDate"></v-list-tile-sub-title>
          </v-list-tile-content>
        </v-list-tile>
      </template>
    </v-list>
    <v-card-text v-if="!profile.trophyList">
      <div class="text-xs-center">
        <v-progress-circular indeterminate color="primary"></v-progress-circular>
      </div>
      <div class="text-xs-center">
        <h2>Your trophy list is updating please check again later</h2>
      </div>
    </v-card-text>
  </v-card>
</template>

<script lang="ts">
import TrophyListItem from "./TrophyListItem.vue";
export default {
  name: "Profile",
  components: {
    TrophyListItem
  },
  data() {
    return {
      interval: {},
      level: 0,
      value: 0
    };
  },
  props: ["profile"],
  beforeDestroy() {
    clearInterval(this.interval);
  },
  mounted() {
    this.interval = setInterval(() => {
      if (!this.profile) {
        return this.value === 0;
      } else if (
        this.value < this.profile.trophySummary.level &&
        this.value < this.profile.trophySummary.progress
      ) {
        this.level += 1;
        this.value += 1;
      } else {
        this.level = this.profile.trophySummary.level;
        this.value = this.profile.trophySummary.progress;
        return this.value === 0;
      }
    }, 2);
  }
};
</script>

<style scoped>
</style>

