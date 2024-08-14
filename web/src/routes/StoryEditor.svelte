<script lang="ts" context="module">
    import _ from "lodash";
</script>
<script lang="ts">
    import { onDestroy } from "svelte";

    export let title: string | undefined;

    interface Story {
        title: string,
        text: string,
        entries: {
            [key: string]: string
        },
        description: string
    }

    let originalStory: Story | undefined;
    let currentStory: Story | undefined;

    let storyPromise = fetch("/api/read", {
        method: "POST",
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(title)
    }).then(r => {
        if (r.status != 200) {
            console.error(r);
        } else {
            return r.json();
        }
    }).then(j => {
        originalStory = j;
        currentStory = _.cloneDeep(j);
    });

    let ms = 1000
 
    let clear: number = setInterval(save, ms);
    onDestroy(() => {
        clearInterval(clear);
    });

    async function save() {
        if (currentStory !== undefined && !_.isEqual(currentStory, originalStory)) {
            await fetch("/api/write", {
                method: "POST",
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(currentStory)
            });
            originalStory = _.cloneDeep(currentStory);
        }
    }
</script>

<button on:click={() => title = undefined}>back</button>
<h1>{title}</h1>
{#await storyPromise}
    Loading story...
{:then}
    {#if currentStory !== undefined && originalStory !== undefined}
        <textarea bind:value={currentStory.text}></textarea>
    {:else}
        Unreachable, story should be defined.
    {/if}
{/await}
