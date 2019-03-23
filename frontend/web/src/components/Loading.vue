<template>
  <div class="text-xs-center">
    <v-progress-circular
      :size="70"
      :width="7"
      color="dark"
      indeterminate
    ></v-progress-circular>
  </div>
</template>

<script lang="ts">
export default {
    name: 'Loading'
}
</script>

<style scoped>
  .v-progress-circular {
      margin-top:16%
  }
</style>