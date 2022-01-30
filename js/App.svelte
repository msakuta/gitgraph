<script>
    import { GitGraph } from "./gitgraph";
    import Details from "./Details.svelte";
    import ToolTip from "./ToolTip.svelte";

    const columnOffset = 20;
    const columnWidth = 15;
    const rowOffset = 10;
    const rowHeight = 20;
    let graphWidth = 100;

    let svg = null;
    let gitgraph = new GitGraph(svg);
    $: gitgraph.svg = svg;

    let branches = [];
    let selectedBranch = null;

    let selectedCommit = null;
    let detailMeta = {};

    let showToolTip = false;
    let tipMeta = {};
    let tipDiffStats = "";
    let tipCommit = "";
    let tipLeft = 0;
    let tipTop = 0;

    function getRefs(){
        fetch("refs")
        .then(resp => resp.json())
        .then(refs => {
            for(const ref in refs){
                branches.push(ref);
            }
            branches = branches;
            const idx = branches.indexOf("refs/heads/master");
            if(0 <= idx){
                selectedBranch = branches[idx];
            }
        });
    }

    function branchChanged(branch){
        if(!branch || branch === null)
            return;
        console.log(`changed: ${branch}`);
        fetch(`/commit-query/${branch}`)
            .then(resp => resp.json())
            .then(({commits, session}) => {
                graphWidth = gitgraph.newSession(commits, session,
                    {
                        showCommit: setTipCommit,
                        showMeta: setTipMeta,
                        showDiffStats: setTipDiffStats,
                        hideMessage: hideTipMessage,
                        showDetails,
                    }
                );
                allCommits = gitgraph.allCommits;
            });
    }

    $: branchChanged(selectedBranch);

    let pendingFetch = null;
    let commitMap = {};
    let allCommits = [];
    let detailFiles = [];

    function setTipCommit(commit, left, top){
        showToolTip = true;
        tipCommit = commit;
        tipLeft = left;
        tipTop = top;
        console.log(`tip: ${tipLeft} ${tipTop}`)
    }

    function setTipMeta(meta){
        tipMeta = meta;
    }

    function setTipDiffStats(diffStats){
        tipDiffStats = diffStats;
    }

    function hideTipMessage(){
        showToolTip = false;
    }

    function showDetails(commit) {
        if(commit.parents.length === 0)
            return;

        detailMeta = {};
        detailFiles = [];

        fetch(`/commits/${commit.hash}/meta`)
            .then(resp => resp.json())
            .then(meta => detailMeta = meta);

        fetch(`/diff/${commit.parents[0]}/${commit.hash}`)
            .then(resp => resp.json())
            .then(diffFiles => {
                selectedCommit = commit;
                detailFiles = diffFiles;
            });
    }

    function scrollHandle(){
        const scrollBottom = graphElem.scrollTop + graphElem.clientHeight;
        // console.log(`scrollBottom ${scrollBottom}/${graphElem.scrollHeight}`);
        if(graphElem.scrollHeight <= scrollBottom){
            // console.log(`fetch chance ${gitgraph.lastCommits}`);
            if(!pendingFetch && gitgraph.lastCommits.length !== 0 && gitgraph.sessionId){
                pendingFetch = true;
                console.log(`Pending fetch for ${gitgraph.lastCommits[0]} started`);
                fetch("/sessions", {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        session_id: gitgraph.sessionId,
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
                        gitgraph.lastCommits.indexOf(item.hash) !== -1 || !commitMap.hasOwnProperty(item.hash));
                    const yOffset = gitgraph.allCommits.length;
                    // this.renderLog(commits, yOffset);
                    gitgraph.parseLog(commits);
                    allCommits = gitgraph.allCommits;
                    graphWidth = gitgraph.updateSvg(svg, commits, yOffset, {
                        showCommit: setTipCommit,
                        showMeta: setTipMeta,
                        showDiffStats: setTipDiffStats,
                        hideMessage: hideTipMessage,
                        showDetails,
                    }, graphWidth);
                    pendingFetch = false;
                    console.log(`Pending fetch for ${gitgraph.lastCommits[0]} ended`);
                });
            }
        }
    }

    function selectCommit(commit){
        selectedCommit = commit;
        showDetails(commit);
    }

    let graphElem;

    $: if(graphElem) graphElem.addEventListener("scroll", scrollHandle);

    getRefs();
</script>

<div class="headerContainer">
    <label>Branch:
        <select bind:value={selectedBranch}>
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

<div class="graphContainer" bind:this={graphElem}>
    <svg bind:this={svg} width="{graphWidth}px" height="400px" style="width: {graphWidth}px; height: 400px;"></svg>

    <div class="messages" style="left: {graphWidth}px">
        {#each allCommits as commit, index}
            <div class={selectedCommit === commit ? 'selected' : index % 2 === 0 ? 'light' : 'dark'}
                style="position: absolute; left: 0px; top:{index * rowHeight - rowHeight / 2 + rowOffset}px; width: 100%; height: {rowHeight}px"
                on:click={selectCommit(commit)}>
                <span class="valign" id={commit.hash}>
                    {commit.hash.substr(0, 6)} {commit.message}
                </span>
            </div>
        {/each}
    </div>
</div>

<Details commit={selectedCommit} meta={detailMeta} files={detailFiles}/>

{#if showToolTip}
<ToolTip {tipCommit} {tipLeft} {tipTop} {tipMeta} {tipDiffStats}/>
{/if}

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
    .selected{
        background-color: #ffffff;
        border: solid 2px #0000ff;
        margin: -4px;
    }
    .dark{
        background-color: #cfcfcf;
    }
    .light{
        background-color: #efefef;
    }
</style>