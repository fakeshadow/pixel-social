<template>
    <v-container grid-list-md text-xs-center>
        <v-layout row wrap fill-height>
            <v-flex v-bind:key="i" v-for="(item,i) in storeItems.items" xs6 sm4 md3 lg2 xl1>
                <StoreItemCard v-bind:item="{region:storeItems.region,detail:item}"/>
            </v-flex>
        </v-layout>
    </v-container>
</template>

<script lang="ts">
    import StoreItemCard from "./StoreItemCard.vue";

    export default {
        name: "StoreItem",
        components: {
            StoreItemCard
        },
        props: ["storeItems"],
        methods: {}
    };
</script>

<style scoped>
</style>