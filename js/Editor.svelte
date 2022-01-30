<script>
    export let caption = "";
    export let editor;
    export let date;
    let dateTime = new Date(date * 1000);
</script>

<div>
    <b>{caption}:</b> {editor.name} &lt;{editor.email}&gt; {dateTime.toLocaleString()}
</div>
