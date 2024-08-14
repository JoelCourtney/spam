<script lang="ts">
	import StoryEditor from "./StoryEditor.svelte";

	let storiesPromise = fetch("/api/list", {method: "POST"}).then(r => r.json());

	let selected: string | undefined = undefined;
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Svelte demo app" />
</svelte:head>

{#if selected == undefined}
	<section id="title">
		<h1>
			Engage Spam
		</h1>
	</section>
	<section>
		{#await storiesPromise}
			<h2>Loading stories...</h2>
		{:then stories} 
			<h2>Choose a story</h2>
			{#each stories as story}
				<button on:click={() => selected = story}>{story}</button>
			{/each}
		{/await}
	</section>
{:else}
	<StoryEditor bind:title={selected} />
{/if}

<style>
	#title {
		flex: 0.1;
	}

	section {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
	}

	h1 {
		width: 100%;
	}
</style>
