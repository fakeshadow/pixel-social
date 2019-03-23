
<template>
  <v-card hover ripple height="100%" @click="modal = true">
    <v-img v-bind:src="item.detail.thumbNail" aspect-ratio="1"></v-img>
    <v-card-title text-xs-center>
      <h5>{{item.detail.name}}</h5>
    </v-card-title>
    <v-flex xs12 justify-center>
      <v-dialog v-model="modal" max-width="600">
        <v-card class="mx-auto" dark max-width="600" tile>
          <v-card-title>
            <v-layout row wrap justify-center text-xs-center offset-sm3>
              <v-flex xs12>
                <v-img v-bind:src="item.detail.thumbNail" contain height="200"></v-img>
              </v-flex>
              <v-flex xs12></v-flex>
              <v-flex xs12>
                <span class="headline font-weight-bold">
                  <br>
                  {{item.detail.name}}
                </span>
              </v-flex>
              <v-spacer></v-spacer>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Type:
                </div>
                <h4>{{item.detail.gameContentType}}</h4>
              </v-flex>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Prices:
                </div>
                <div>
                  <h4 class="yellow--text">
                    {{item.detail.prices.plus.price / 100}} {{moneySymbol}}
                    <br>
                  </h4>
                </div>
                <h4>{{item.detail.prices.noPlus.price / 100}} {{moneySymbol}}</h4>
              </v-flex>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Release Date:
                </div>
                <h4>{{item.detail.releaseDate}}</h4>
              </v-flex>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Publisher:
                </div>
                <h4>{{item.detail.provider}}</h4>
              </v-flex>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Genre:
                </div>
                <div v-for="(genre,i) in item.detail.genres" :key="i">
                  <h4>
                    {{genre}}
                    <br>
                  </h4>
                </div>
              </v-flex>
              <v-flex xs4>
                <div class="subheading font-weight-light">
                  <br>Rating:
                </div>
                <h4>Score: {{item.detail.starRating.score}} | Total: {{item.detail.starRating.total}}</h4>
              </v-flex>
            </v-layout>
          </v-card-title>
          <v-carousel
            v-if="item.detail.mediaList.screenshots.length"
            height="auto"
            hide-delimiters
            :cycle="false"
          >
            <v-carousel-item
              v-for="screenshot of item.detail.mediaList.screenshots"
              v-bind:key="screenshot.url"
            >
              <v-img v-bind:src="screenshot.url"></v-img>
            </v-carousel-item>
          </v-carousel>
          <v-card-text v-html="item.detail.description">
          </v-card-text>

          <v-card-actions>
            <v-list-tile class="grow">
              <v-layout align-center justify-end>
                <v-icon class="mr-1">mdi-heart</v-icon>
                <span class="subheading mr-2">{{item.detail.starRating.score}}</span>
                <span class="mr-1">·</span>
                <v-icon class="mr-1">md-share-variant</v-icon>
                <span class="subheading">{{item.detail.starRating.total}}</span>
                <v-btn color="blue" flat="flat" @click="openPSNStore">Go to Store</v-btn>
              </v-layout>
            </v-list-tile>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </v-flex>
  </v-card>
</template>

<script lang="ts">
export default {
  name: "StoreItemCard",
  props: ["item"],
  data() {
    return {
      modal: false,
      moneySymbol: '$'
    };
  },
  mounted() {
    switch(this.item.region) {
      case 'US':
        this.moneySymbol = '$'
            break;
      case 'UK':
        this.moneySymbol = '£'
            break;
      case 'HK':
        this.moneySymbol = '$'
            break;
    }
  },
  methods: {
    openPSNStore() {}
  }
};
</script>
