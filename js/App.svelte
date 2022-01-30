<script>
    import _$ from "jquery";
    import { GitGraph } from "./gitgraph";
    import Details from "./Details.svelte";

    const columnOffset = 20;
    const columnWidth = 15;
    const rowOffset = 10;
    const rowHeight = 20;
    let graphWidth = 100;

    let svg = null;
    let gitgraph = null;
    $: gitgraph = new GitGraph(svg);

    let branches = [];

    function getRefs(){
        var commitsAjax = _$.get("commit-query")
        var refsAjax = _$.get("refs")
        _$.when(commitsAjax, refsAjax)
        .then(function(response, refs){
            const {commits, session} = response[0];
            // gitgraph.refs = refs[0];
            for(const ref in refs[0]){
                branches.push(ref);
            }
            branches = branches;
        // }
        // newSession(commits, session);
        });
    }

    let commits = [];

    function renderLog(aCommits, yOffset=0){
        commits = aCommits;
    }

    function branchChanged(event){
        console.log(`changed: ${event.target.value}`);
        fetch(`/commit-query/${event.target.value}`)
            .then(resp => resp.json())
            .then(({commits, session}) => {
                renderLog(commits);
                graphWidth = gitgraph.newSession(commits, session, aMessage => message = aMessage);
            });
    }

    let pendingFetch = null;
    let lastCommits = [];
    let sessionId = "";
    let commitMap = {};
    let allCommits = [];
    let message = [];

    function scrollHandle(){
        const scrollBottom = _$(window).scrollTop() + document.documentElement.clientHeight;
        // console.log(`scrollBottom ${scrollBottom}/${document.body.scrollHeight}`);
        if(document.body.scrollHeight <= scrollBottom){
            if(!pendingFetch && lastCommits.length !== 0 && sessionId){
                pendingFetch = true;
                console.log(`Pending fetch for ${lastCommits[0]} started`);
                fetch("/sessions", {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        session_id: sessionId,
                    }),
                })
                .then((resp) => {
                    if(!resp.ok){
                        throw new Error(`HTTP error! status: ${resp.status}`);
                    }
                    return resp.json();
                })
                .then(json => {
                    const commits = json.filter(item =>
                        lastCommits.indexOf(item.hash) !== -1 || !commitMap.hasOwnProperty(item.hash));
                    const yOffset = allCommits.length;
                    // this.renderLog(commits, yOffset);
                    gitgraph.parseLog(commits);
                    graphWidth = gitgraph.updateSvg(svg, commits, yOffset, (aMessage) => message = aMessage);
                    pendingFetch = false;
                    console.log(`Pending fetch for ${this.lastCommits[0]} ended`);
                });
            }
        }
    }

    _$(window).scroll(scrollHandle);

    getRefs();
</script>

<div class="headerContainer">
    <label>Branch:
        <select on:change={branchChanged}>
            {#each branches as branch}
            <option>{branch}</option>
            {/each}
        </select>
        <button on:click={getRefs}>Get refs</button>
    </label>
    <div style="text-align: center">Powered by 
        <a href="https://github.com/sveltejs/svelte">Svelte</a>
    </div>
</div>

<div class="graphContainer">
    <svg bind:this={svg} width="{graphWidth}px" height="400px" style="width: {graphWidth}px; height: 400px;"></svg>

    <div class="messages" style="left: {graphWidth}px">
        {#each commits as commit, index}
            <div class={index % 2 === 0 ? 'light' : 'dark'}
                style="position: absolute; left: 0px; top:{index * rowHeight - rowHeight / 2 + rowOffset}px; width: 100%; height: {rowHeight}px">
                <span class="valign" id={commit.hash}>
                    {commit.hash.substr(0, 6)} {commit.message}
                </span>
            </div>
        {/each}
    </div>
</div>

<Details message={message}/>


<style>
    .headerContainer{
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 10%;
    }
    .graphContainer{
        position: fixed;
        top: 10%;
        left: 0;
        width: 100%;
        height: 40%;
        overflow-y: scroll;
	}
</style>