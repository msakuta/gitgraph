<script>
    import Editor from "./Editor.svelte";
    export let commit;
    export let meta = {};
    export let message = [];
</script>

<div class="details">
    {#if commit}
        <div style="font-familiy: monospace"><b>Commit: </b>{commit.hash}</div>
        {#if meta.author}
            <Editor caption="Author" editor={meta.author} date={meta.author.date}/>
        {/if}
        {#if meta.committer}
            <Editor caption="Committer" editor={meta.committer} date={meta.committer.date}/>
        {/if}
        {#if meta.message}
        <div style="font-familiy: monospace">
            <pre>{meta.message}</pre>
        </div>
        {:else}
        <p>{commit.message}</p>
        {/if}
    {/if}

    Hunks: {message.length}

    <div class="hunk" style="position: relative; text-align: center">
        {#each message as row, i}
            <div style="text-align: left; background-color: #ffffcf; margin: 4px">
                <pre>
                    {row}
                </pre>
            </div>
        {/each}
    </div>
</div>

<style>
    .details{
        position: fixed;
        bottom: 0;
        left: 0;
        width: 100%;
        height: 50%;
        overflow-y: scroll;
        font-family: monospace;
        background-color: #cfcf9f;
        border: solid 2px #3f3fff;
	}
    .hunk {
        text-align: left;
    }
</style>