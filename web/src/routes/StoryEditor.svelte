<script lang="ts" context="module">
    import _ from "lodash";
    import sc from "string-comparison";
</script>

<script lang="ts">
    import { onDestroy } from "svelte";
    import { generationStream } from "../generate";
    import type { Story } from "../generate";

    import {listener, listenerCtx} from '@milkdown/plugin-listener'
    import { Crepe, CrepeFeature } from '@milkdown/crepe';
    import { replaceAll, getMarkdown } from "@milkdown/utils";


    import '@milkdown/crepe/theme/common/style.css'
    import '@milkdown/crepe/theme/nord-dark.css';

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
    }).then(async j => {
        originalStory = j;
        currentStory = _.cloneDeep(j);
        await reconcileImages();
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
    ).then(j => {
        let data = j["data"];
        let result: any = {};
        for (const m of data) {
            result[m.name] = m;
        }
        return result;
    });

    async function filteredModels(search: string): Promise<string[]> {
        let ms = await models;
        let ids = Object.keys(ms);

        let sorted = sc.jaroWinkler.sortMatch(search, ids)
            .map(r => r.member);

        return sorted.slice(-10);
    }

    async function cost(model: string): Promise<number | undefined> {
        let ms = await models;
        let result = ms[model];
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
        if (currentStory !== undefined) {
            if (originalStory !== undefined && currentStory.title !== originalStory.title) {
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
            if (!_.isEqual(currentStory, originalStory)) {
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
    }

    let generationAbort: AbortController | undefined = undefined;
    let textBeforeGeneration: string | undefined;

    let dragover: boolean = false;
    let fileInput: HTMLInputElement;
    let uploadStatus: string | undefined = undefined;

    let showImages: boolean = false;

    async function reconcileImages() {
        if (currentStory === undefined) return;
        const res = await fetch("/api/assets/list", {
            method: "POST",
            headers: { 'Accept': 'application/json', 'Content-Type': 'application/json' },
            body: JSON.stringify(currentStory.title)
        });
        const diskFiles: string[] = res.ok ? await res.json() : [];
        const existing = currentStory.images ?? [];
        const kept = existing.filter(name => diskFiles.includes(name));
        const added = diskFiles.filter(name => !kept.includes(name));
        currentStory.images = [...kept, ...added];
    }

    async function toggleImages() {
        showImages = !showImages;
        if (showImages) {
            await reconcileImages();
        }
    }

    async function uploadFile(file: File): Promise<boolean> {
        if (currentStory === undefined) return false;
        const res = await fetch(
            `/api/upload/${encodeURIComponent(currentStory.title)}/${encodeURIComponent(file.name)}`,
            {
                method: "POST",
                headers: { 'Content-Type': 'application/octet-stream' },
                body: file
            }
        );
        if (!res.ok) {
            console.error("upload failed", file.name, res.status);
            return false;
        }
        return true;
    }

    async function uploadFiles(files: FileList | File[] | null | undefined) {
        if (!files || files.length === 0) return;
        if (currentStory === undefined) return;
        const images = Array.from(files).filter(f => f.type.startsWith("image/"));
        if (images.length === 0) {
            uploadStatus = "no image files";
            return;
        }
        uploadStatus = `uploading ${images.length}...`;
        for (const file of images) {
            const ok = await uploadFile(file);
            if (ok && !currentStory.images.includes(file.name)) {
                currentStory.images = [...currentStory.images, file.name];
            }
        }
        uploadStatus = `uploaded ${images.length}`;
        setTimeout(() => uploadStatus = undefined, 2000);
    }

    async function deleteImage(name: string) {
        if (currentStory === undefined) return;
        const res = await fetch(
            `/api/assets/delete/${encodeURIComponent(currentStory.title)}/${encodeURIComponent(name)}`,
            { method: "POST" }
        );
        if (!res.ok && res.status !== 404) {
            console.error("delete failed", name, res.status);
            return;
        }
        currentStory.images = currentStory.images.filter(n => n !== name);
    }

    function moveImage(index: number, delta: number) {
        if (currentStory === undefined) return;
        const next = [...currentStory.images];
        const target = index + delta;
        if (target < 0 || target >= next.length) return;
        [next[index], next[target]] = [next[target], next[index]];
        currentStory.images = next;
    }

    function handleDrop(e: DragEvent) {
        dragover = false;
        uploadFiles(e.dataTransfer?.files);
    }

    function handleFileInput(e: Event) {
        const input = e.target as HTMLInputElement;
        uploadFiles(input.files);
        input.value = "";
    }

    async function generate() {
        if (currentStory !== undefined) {
            currentStory.text = editorInstance.action(getMarkdown());
            textBeforeGeneration = currentStory.text;
            try {
                let ms = await models;
                let generation = await generationStream(currentStory, ms[currentStory.model].id, await key, await promptTemplate);
                generationAbort = generation.abort;
                for await (const chunk of generation.stream) {
                    currentStory.text += chunk;
                    editorInstance.action(replaceAll(currentStory.text));
                    scrollToBottom();
                }
            } catch (e) {
                console.log(e);
            } finally {
                generationAbort = undefined;
            }
        }
    }

    function undoGeneration() {
        if (textBeforeGeneration !== undefined) {
            editorInstance.action(replaceAll(textBeforeGeneration));
            currentStory.text = textBeforeGeneration;
            textBeforeGeneration = undefined;
        }
    }

    function scrollToBottom() {
        let editorArea = document.getElementsByClassName("editor")[0];
        editorArea.scrollIntoView(false);
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

    let editorInstance: Editor;

    function editor(dom: any) {
        const crepe = new Crepe({
            root: dom,
            defaultValue: currentStory?.text,
            featureConfigs: {
                [CrepeFeature.Placeholder]: {
                    text: 'write...',
                    mode: 'block'
                }
            }
        });
        crepe.editor.config((ctx) => {
            ctx.get(listenerCtx).markdownUpdated((ctx: any, markdown: string, _: string) => {
                if (currentStory !== undefined && generationAbort === undefined) currentStory.text = markdown;
            })
        }).use(listener);
        crepe.create()
            .then(e => {
                editorInstance = e;
                scrollToBottom();
            });
    }
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
                    <div class="h-100 col-8" id="maindiv">
                        <div use:editor class="h-100"/>
                        <!-- <textarea id="main" placeholder="spam" class="form-control h-100" rows="6" bind:value={currentStory.text}></textarea> -->
                    </div>
                    {#if showImages}
                        <div class="h-100 col-3 images-col">
                            {#if currentStory.images.length === 0}
                                <div class="text-center text-muted py-3">no images</div>
                            {:else}
                                {#each currentStory.images as asset (asset)}
                                    <img
                                        class="story-img"
                                        src={`/api/assets/${encodeURIComponent(currentStory.title)}/${encodeURIComponent(asset)}`}
                                        alt={asset}
                                    />
                                {/each}
                            {/if}
                        </div>
                    {/if}
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
                    <div class="col-8">
                        <h2>images</h2>
                        {#if currentStory.images.length > 0}
                            <ul class="list-group list-group-flush">
                                {#each currentStory.images as img, i (img)}
                                    <li class="list-group-item d-flex align-items-center gap-2">
                                        <img
                                            class="image-thumb"
                                            src={`/api/assets/${encodeURIComponent(currentStory.title)}/${encodeURIComponent(img)}`}
                                            alt={img}
                                        />
                                        <span class="flex-grow-1 text-truncate">{img}</span>
                                        <div class="btn-group" role="group">
                                            <button class="btn btn-sm btn-outline-secondary" on:click={() => moveImage(i, -1)} disabled={i === 0} aria-label="move up">↑</button>
                                            <button class="btn btn-sm btn-outline-secondary" on:click={() => moveImage(i, 1)} disabled={i === currentStory.images.length - 1} aria-label="move down">↓</button>
                                            <button class="btn btn-sm btn-outline-danger" on:click={() => deleteImage(img)} aria-label="delete">delete</button>
                                        </div>
                                    </li>
                                {/each}
                            </ul>
                        {/if}
                        <div
                            class="image-drop-zone"
                            class:dragover
                            role="button"
                            tabindex="0"
                            on:click={() => fileInput.click()}
                            on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') fileInput.click(); }}
                            on:dragover|preventDefault={() => dragover = true}
                            on:dragenter|preventDefault={() => dragover = true}
                            on:dragleave={() => dragover = false}
                            on:drop|preventDefault={handleDrop}
                        >
                            {#if uploadStatus !== undefined}
                                {uploadStatus}
                            {:else}
                                drop images here or click to select
                            {/if}
                        </div>
                        <input
                            type="file"
                            accept="image/*"
                            multiple
                            style="display: none;"
                            bind:this={fileInput}
                            on:change={handleFileInput}
                        />
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
                                    <button class="btn btn-outline-danger" type="button" id="spam-button" on:click={() => generationAbort?.abort()}>stop</button>
                                {/if}
                            </div>
                        </div>
                    </div>
                    <div class="col-2 text-center">
                        <div class="btn-group" role="group">
                            <button class="btn btn-outline-primary" on:click={() => {generationAbort?.abort(); undoGeneration(); generate();}} style="height: 58px" disabled={textBeforeGeneration === undefined}>retry</button>
                            <button class="btn btn-outline-primary" on:click={undoGeneration} disabled={textBeforeGeneration === undefined}>undo</button>
                        </div>
                    </div>
                    <div class="col-1 text-center">
                        <div class="btn-group" role="group">
                            <button class="btn btn-outline-primary" style="height:58px;" on:click={toggleImages}>
                                {showImages ? "hide" : "images"}
                            </button>
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
        
        line-height: 150%;
        font-size: 17px;
    }
    
    div.extras {
        padding-top: 12px;
        padding-bottom: 12px;
    }

    .image-drop-zone {
        border: 2px dashed #6c757d;
        border-radius: 6px;
        padding: 32px;
        text-align: center;
        color: #adb5bd;
        cursor: pointer;
        transition: border-color 0.15s ease-in-out, color 0.15s ease-in-out, background-color 0.15s ease-in-out;
    }

    .image-drop-zone:hover,
    .image-drop-zone.dragover {
        border-color: #0d6efd;
        color: #0d6efd;
        background-color: rgba(13, 110, 253, 0.08);
    }

    .images-col {
        overflow-y: auto;
    }

    .story-img {
        display: block;
        width: 100%;
        height: auto;
        border-radius: 6px;
        margin-bottom: 12px;
    }

    .story-img:last-child {
        margin-bottom: 0;
    }

    .image-thumb {
        width: 48px;
        height: 48px;
        object-fit: cover;
        border-radius: 4px;
        flex-shrink: 0;
    }

</style>
