<script>
    import Editor from "./Editor.svelte";
    import Foldable from "./Foldable.svelte";
    export let commit;
    export let meta = {};
    export let files = [];
    let defaultVisible = false;
    $: if(commit && commit.stat){
        defaultVisible = (commit.stat.insertions + commit.stat.deletions) < 20;
        console.log(`stats: ${commit.stat.insertions} ${commit.stat.deletions} ${defaultVisible}`);
    }
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

    Files: {files.length}

    <div class="hunk">
        {#each files as file}
            <Foldable {defaultVisible}>
                <div slot="header" class="fileHeader">{file.file}</div>
                <div slot="content" style="text-align: left; background-color: #ffffcf; margin: 4px">
                    {#each file.hunks as hunk}
                    <pre>
                        {hunk}
                    </pre>
                    {/each}
                </div>
            </Foldable>
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
        position: relative;
    }
    .fileHeader:hover {
        background-color: #ffffcf;
    }
</style>