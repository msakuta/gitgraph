<script>
    export let message;
</script>

<div class="details">
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