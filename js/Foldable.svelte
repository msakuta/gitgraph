<script>
    export let defaultVisible = false;
    let visible = defaultVisible;

    function toggleVisible(){
        visible = !visible;
    }
</script>

<div style="border: solid 1px #7f7fff; margin: 2px">
    <div class="header" on:click={toggleVisible} style="background-color: #afaf8f">
        <slot name="header"></slot>
    </div>
    {#if visible}
        <slot name="content"></slot>
    {/if}
</div>

<style>
</style>