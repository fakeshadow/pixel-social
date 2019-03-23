<template>
    <div @click="getListDetail(onlineId, listItem.npCommunicationId)">
        <p>{{listItem.npCommunicationId}}</p>
        <p>{{listItem.progress}}</p>
        <p>{{listItem.earnedTrophies}}</p>
        <p>{{listItem.lastUpdateDate}}</p>
    </div>
</template>

<script lang="ts">
    export default {
        name: "TrophyListItem",
        props: ["onlineId", "listItem"],
        methods: {
            async getListDetail(onlineId, npCommunicationId) {
                const response = await fetch(process.env.VUE_APP_PSNURL + 'trophy', {
                    method: 'post',
                    body: JSON.stringify({onlineId, npCommunicationId}),
                    headers: {'Content-Type': 'application/json'}
                })
                const listDetail = response.json()
            }
        }
    };
</script>

<style scoped>
</style>

