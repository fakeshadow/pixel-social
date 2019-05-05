<head>
	<link rel="stylesheet" href="https://stackpath.bootstrapcdn.com/bootstrap/4.3.1/css/bootstrap.min.css" integrity="sha384-ggOyR0iXCbMQv3Xipma34MD+dH/1fQ784/j6cY/iJTQUOhcWr7x9JvoRxT2MZw1T" crossorigin="anonymous">
</head>

<script>
	import { fade, fly } from 'svelte/transition';
	export let name;
	let alert = false;

	
	function toggleAlert() {
		alert = !alert;
	}

</script>

<style>
	h1 {
		color: purple;
	}
</style>

<h1>Hello {name}!</h1>

<hr>

{#if alert}
<div class="alert alert-primary" role="alert" in:fly="{{ y: 200, duration: 2000 }}" out:fade>
  A simple primary alert—check it out!
</div>
{/if}

<div class="card" style="width: 18rem;">
  <img src="..." class="card-img-top" alt="test">
  <div class="card-body">
    <h5 class="card-title">Card title</h5>
    <p class="card-text">Some quick example text to build on the card title and make up the bulk of the card's content.</p>
    <a href="#" class="btn btn-primary" on:click={toggleAlert}>Go somewhere</a>
  </div>
</div>

<h1>
 Clicked: {alert}
</h1>