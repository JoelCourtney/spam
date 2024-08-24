<script lang="ts">
	import StoryEditor from "./StoryEditor.svelte";

	let storiesPromise: Promise<any>;
	function getStories() {
		storiesPromise = fetch("/api/list", {method: "POST"}).then(r => r.json());
	}

	let selected: string | undefined = undefined;

	$: if (selected === undefined) {
		getStories();
		newStoryTitle = "";
	}

	let newStoryTitle = "";

	async function newStory() {
		await fetch("/api/new", {
			method: "POST",
			headers: {
				'Accept': 'application/json',
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(newStoryTitle)
		});
		selected = newStoryTitle;
	}
</script>

<svelte:head>
	<title>{selected == undefined ? "Home" : selected}</title>
	<meta name="description" content="Svelte demo app" />
</svelte:head>

<div class="container-fluid h-100 montserrat-regular">
	{#if selected == undefined}
        <div class="row justify-content-center">
			<div class="col-9">
				<h1 class="display-1">
					commence spam
				</h1>
			</div>
			{#await storiesPromise}
				<h2>connecting...</h2>
			{:then stories} 
				<div class="col-5">
					<h2>choose a story</h2>
					<div class="list-group list-group-flush">
						{#each stories as story}
							<button class="list-group-item list-group-item-action" on:click={() => selected = story}>{story}</button>
						{/each}
					</div>
				</div>
				<div class="col-4">
					<div class="row">
						<h2>new story</h2>
					</div>
					<div class="row">
						<div class="input-group">
							<div class="form-floating">
								<input type="text" placeholder="title" class="form-control" aria-describedby="button-addon2" id="new-title" bind:value={newStoryTitle}>
								<label for="new-title">title</label>
							</div>
							<button class="btn btn-outline-primary" type="button" id="button-addon2" on:click={newStory}>create</button>
						</div>
					</div>
				</div>
			{/await}
		</div>
	{:else}
		<StoryEditor bind:title={selected} />
	{/if}
</div>

<style>
	input#new-title {
		box-shadow: none !important;
	}
</style>
