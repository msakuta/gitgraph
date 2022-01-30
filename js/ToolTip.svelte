<script>
    import Editor from "./Editor.svelte";
    export let tipCommit = {};
    export let tipLeft = 0;
    export let tipTop = 0;
    export let tipMeta = {};
    export let tipDiff = "";
    let tipMessageElem;
    let tipDiffElem;
</script>

<div class="tipElem" style="position: absolute; left: {tipLeft}px; top: {tipTop}px;">
    <div style="font-familiy: monospace"><b>Commit: </b>{tipCommit.hash}</div>
    {#if tipMeta.author}
        <Editor caption="Author" editor={tipMeta.author} date={tipMeta.author.date}/>
    {/if}
    {#if tipMeta.committer}
        <Editor caption="Committer" editor={tipMeta.committer} date={tipMeta.committer.date}/>
    {/if}
    {#if tipMeta.message}
    <div bind:this={tipMessageElem} style="font-familiy: monospace">
        <pre>{tipMeta.message}</pre>
    </div>
    {:else}
    <p>{tipCommit.message}</p>
    {/if}
    <div bind:this={tipDiffElem} style="font-familiy: monospace">{tipDiff}</div>
</div>

<style>
    .tipElem {
        font-family: monospace;
        padding: 0.5em;
        background-color: #cfcf9f;
        border: solid 2px #3f3fff;
        position: absolute;
        pointer-events: none;
    }
</style>