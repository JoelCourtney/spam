<script lang="ts" context="module">
    import _ from "lodash";
    import sc from "string-comparison";
</script>
<script lang="ts">
    import { onDestroy } from "svelte";
    import { generationStream } from "../generate";
    import type { Story, Generation } from "../generate";

    export let title: string | undefined;

    let originalStory: Story | undefined;
    let currentStory: Story | undefined;

    let mainView: boolean = true;
    let selectedEntry: string | undefined = undefined;
    let entryKeyField: string | undefined = undefined;
    
    let deleteTriggered: boolean = false;

    async function deleteStory() {
        await fetch("/api/delete", {
            method: "POST",
            headers: {
                'Accept': 'application/json',
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(title)
        });
        title = undefined;
    }

    function entryKeyFieldUpdate() {
        if (selectedEntry !== undefined && entryKeyField !== undefined && currentStory !== undefined) {
            delete Object.assign(currentStory.entries, {[entryKeyField]: currentStory.entries[selectedEntry] })[selectedEntry];
            currentStory = currentStory;
            selectedEntry = entryKeyField;
        }
    }

    function entryDelete() {
        if (selectedEntry !== undefined && entryKeyField !== undefined && currentStory !== undefined) {
            delete currentStory.entries[selectedEntry];
            currentStory = currentStory;
            selectedEntry = undefined;
            entryKeyField = undefined;
        }
    }

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

    let key = fetch("/api/key", {
        method: "POST",
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        },
    }).then(r => {
        if (r.status != 200) {
            console.error(r);
        } else {
            return r.json();
        }
    });

    let models = fetch("https://openrouter.ai/api/v1/models").then(
        r => {
            if (r.status !== 200) {
                console.error(r);
            } else {
                return r.json();
            }
        }
    ).then(j => j["data"]);

    async function filteredModels(search: string): Promise<string[]> {
        let ms = await models;
        let ids = ms.map((m: any) => m.id);

        let sorted = sc.jaroWinkler.sortMatch(search, ids)
            .map(r => r.member);

        return sorted.slice(-10);
    }

    async function cost(model: string): Promise<number | undefined> {
        let ms = await models;
        let result = ms.find((m: any) => m.id === model);
        if (result !== undefined) {
            return Number(result.pricing.completion) * 1_000_000;
        }
        return undefined;
    }

    let ms = 1000
 
    let clear: number = setInterval(save, ms);
    onDestroy(() => {
        clearInterval(clear);
    });

    async function save() {
        if (currentStory !== undefined && originalStory !== undefined && currentStory.title !== originalStory.title) {
            await fetch("/api/rename", {
                method: "POST",
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    from: originalStory.title,
                    to: currentStory.title
                })
            });
            originalStory.title = currentStory.title;
        }
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

    let generationAbort: AbortController | undefined = undefined;

    async function generate() {
        if (currentStory !== undefined) {
            try {
                let generation = await generationStream(currentStory, await key, await promptTemplate);
                generationAbort = generation.abort;
                for await (const chunk of generation.stream) {
                    currentStory.text += chunk;
                }
            } catch (e) {
                console.log(e);
            } finally {
                generationAbort = undefined;
            }
        }
    }

    let promptTemplate = fetch("/api/prompt", {
        method: "POST",
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        }
    }).then(
        r => {
            if (r.status !== 200) {
                console.error(r);
            } else {
                return r.text();
            }
        }
    );
</script>

<div class="container-fluid" style="height: 10%">
    <div class="row align-items-center h-100">
        <div class="col-2 text-center">
            <button type="button" class="btn btn-outline-primary" on:click={() => title = undefined}>back</button>
        </div>
        <div class="col-8">
            {#if mainView}
                <h1>{title}</h1>
            {:else if currentStory !== undefined}
                <div class="input-group">
                    <input type="text" class="form-control" aria-describedby="button-addon2" bind:value={currentStory.title} />
                    <button class="btn btn-outline-danger" type="button" id="button-addon2" on:click={
                        () => {
                            if (deleteTriggered) {
                                deleteStory();
                            } else {
                                deleteTriggered = true;
                            }
                        }
                    } on:focusout={() => {deleteTriggered=false;}}>{ !deleteTriggered ? "delete" : "confirm delete" }</button>
                </div>
            {/if}
        </div>
        <div class="col-2 text-center">
            <button type="button" class="btn btn-outline-primary" on:click={() => mainView = !mainView}>{#if mainView}more{:else}story{/if}</button>
        </div>
    </div>
</div>
{#await storyPromise}
    loading story...
{:then}
    {#if currentStory !== undefined && originalStory !== undefined}
        <div class="container-fluid" style="height: 80%">
            {#if mainView}
                <div class="row justify-content-center h-100" style="padding: none;">
                    <div class="h-100 col-8">
                        <textarea id="main" placeholder="spam" class="form-control h-100" rows="6" bind:value={currentStory.text}></textarea>
                    </div>
                </div>
            {:else}
                <div class="row extras justify-content-center">
                    <div class="col-4">
                        <div class="form-floating">
                            <textarea class="form-control" id="floatingDesc" placeholder="description" style="height: 200px" bind:value={currentStory.description}></textarea>
                            <label for="floatingDesc">description</label>
                        </div>
                    </div>
                    <div class="col-4">
                        <div class="form-floating">
                            <textarea class="form-control" id="floatingInstruction" placeholder="instruction" style="height: 200px" bind:value={currentStory.instruction}></textarea>
                            <label for="floatingInstruction">instruction</label>
                        </div>
                    </div>
                </div>
                <div class="row extras justify-content-center">
                    <div class="col-4">
                        <div class="row extras justify-content-between">
                            <div class="col-4">
                                <h2>entries</h2>
                            </div>
                            <div class="col-4 text-end">
                                <button class="btn btn-outline-primary" on:click={() => {
                                    if (currentStory !== undefined && currentStory.entries[""] === undefined) {
                                        currentStory.entries[""] = "";
                                    }
                                    selectedEntry = "";
                                    entryKeyField = "";
                                }}>new</button>
                            </div>
                        </div>
                        <div class="row extras">
                            <div class="list-group list-group-flush">
                                {#each Object.keys(currentStory.entries).toSorted() as entry}
                                    <button class="list-group-item list-group-item-action" on:click={
                                        () => {
                                            selectedEntry = entry;
                                            entryKeyField = entry;
                                        }
                                    }>{entry}</button>
                                {/each}
                            </div>
                        </div>
                    </div>
                    <div class="col-4">
                        {#if selectedEntry !== undefined}
                            <div class="row extras">
                                <div class="col-12">
                                    <div class="form-floating">
                                        <input class="form-control" placeholder="entrykey" id="floatingEntryTitle" bind:value={entryKeyField} on:input={entryKeyFieldUpdate}/>
                                        <label for="floatingEntryTitle">entry key</label>
                                    </div>
                                </div>
                            </div>
                            <div class="row extras">
                                <div class="col-12">
                                    <div class="form-floating">
                                        <textarea class="form-control h-100" placeholder="entrytext" id="floatingEntryText" rows="15" bind:value={currentStory.entries[selectedEntry]}></textarea>
                                        <label for="floatingEntryText">entry text</label>
                                    </div>
                                </div>
                            </div>
                            <div class="row extras justify-content-center">
                                <div class="col-4">
                                    <button class="btn btn-outline-danger" on:click={entryDelete}>delete</button>
                                </div>
                            </div>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>
        {#if mainView}
            <div class="container-fluid" style="height: 10%">
                <div class="row align-items-center justify-content-center h-100">
                    <div class="col-4 text-center">
                        <div class="dropdown">
                            <div class="input-group">
                                <div class="form-floating">
                                    <input bind:value={currentStory.model} id="model-input" class="form-control dropdown-toggle" type="text" data-bs-toggle="dropdown" placeholder="model" aria-label="model" aria-describedby="spam-button"/>
                                    <label for="model-input">model {#await cost(currentStory.model) then c}{#if c !== undefined}(cost: ${c} / M){/if}{/await}</label>
                                    <ul class="dropdown-menu">
                                        {#await filteredModels(currentStory.model) then models}
                                            {#each models as model}
                                                <li><button class="dropdown-item" on:click={() => {if (currentStory !== undefined) currentStory.model = model;}}>{model}</button></li>
                                            {/each}
                                        {/await}
                                    </ul>
                                </div>
                                {#if generationAbort === undefined}
                                    <button class="btn btn-outline-primary" type="button" id="spam-button" on:click={generate}>spam</button>
                                {:else}
                                    <button class="btn btn-outline-danger" type="button" id="spam-button" on:click={() => generationAbort.abort()}>stop</button>
                                {/if}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        {/if}
    {:else}
        Unreachable, story should be defined.
    {/if}
{/await}

<style>
    textarea, input {
        resize: none;
        box-shadow: none !important;
        
        line-height: 180%;
        font-size: 17px;
    }
    
    textarea#main {
        border-radius: 0px;
        border: none !important;
        outline: none !important;
    }

    div.extras {
        padding-top: 12px;
        padding-bottom: 12px;
    }
</style>
