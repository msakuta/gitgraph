window.gitgraph = new (function(){
'use strict'

// Polyfill almost only for IE
Math.log10 = Math.log10 || function(x){
	return Math.log(x) / Math.LN10
}

var NS="http://www.w3.org/2000/svg";

var columnOffset = 20
var columnWidth = 15
var rowOffset = 10
var rowHeight = 20

var commitMap = {}
var allCommits = []
var refs = {}
let lastCommits = []

/** Creates and returns rect element as a SVG element.
 *
 * @param {number} x   X coordinate of left edge.
 * @param {number} y   Y coordinate of top edge.
 * @param {number} width   Width of the rectangle.
 * @param {number} height   Height of the rectangle.
 * @param {string=} color   Fill style of the circle. Default is "white".
 * @return {Element} the SVG element.
 */
function rect(x,y,width,height,color){
	var c = document.createElementNS(NS,"rect");
	c.x.baseVal.value = x;
	c.y.baseVal.value = y;
	c.width.baseVal.value = width;
	c.height.baseVal.value = height;
	c.style.stroke = "#000000";
	c.style.strokeWidth = "1";
	c.style.fill = color || "white";
	return c;
}

/** Creates and returns circle element as a SVG element.
 *
 * @param {number} cx   X coordinate of center point.
 * @param {number} cy   Y coordinate of center point.
 * @param {number} r   Radius of the circle.
 * @param {string=} fill   Fill style of the circle. Default is "white".
 * @param {string=} stroke   Stroke style of the circle. Default is "black".
 * @param {string=} width   strokeWidth style of the circle. Default is "1".
 * @return {Element} the SVG element.
 */
function circle(cx,cy,r,fill,stroke,width){
	var c = document.createElementNS(NS,"circle");
	c.cx.baseVal.value = cx;
	c.cy.baseVal.value = cy;
	c.r.baseVal.value = r;
	c.style.stroke = stroke || "black";
	c.style.strokeWidth = width || "1";
	c.style.fill = fill || "white";
	return c;
}

/** Creates and returns arc element as a SVG element.
 *
 * Since arcs in SVG paths are not straightforward to define,
 * we want a concise function to create an arc, just as simple
 * as creating a circle.
 *
 * @param {number} cx   X coordinate of center point.
 * @param {number} cy   Y coordinate of center point.
 * @param {number} r   Radius of the arc.
 * @param {number} start  Starting angle, in radians, from Y+ axis
 *        in counterclockwise.
 * @param {number} end   Ending angle, in radians, same definition
 *        as start.
 * @param {string} stroke  Stroke style of the path.
 * @return {Element} the SVG element.
 */
function arc(cx,cy,r,start,end,stroke){
	var a = document.createElementNS(NS,"path")
	var startPoint = [cx + r * Math.sin(start), cy - r * Math.cos(start)]
	var endPoint = [cx + r * Math.sin(end), cy - r * Math.cos(end)]
	a.setAttribute('d', "M" + startPoint[0] + " " + startPoint[1] +
		"A" + r + " " + r + " 0 0 1 " +
		endPoint[0] + " " + endPoint[1])
	a.style.stroke = stroke
	a.style.strokeWidth = "4"
	a.style.fill = "none"
	return a
}

function setArrow(a,child,parent){
	var str = "M"
	var maxX = (child.x + 1.5) * columnWidth
	// Accumulate maximum x value
	function xMax(x){
		if(maxX < x) maxX = x
		return x
	}
	if(child.x < parent.x)
		str += xMax(child.x * columnWidth + columnOffset + 7) + "," + (child.y) + "L" +
			xMax(parent.x * columnWidth + columnOffset - 5) + "," + child.y + "," +
			xMax(parent.x * columnWidth + columnOffset) + "," + (child.y + 5) + ","
	else if(parent.x < child.x)
		str += xMax(child.x * columnWidth + columnOffset - 7) + "," + (child.y) + "L" +
			xMax(parent.x * columnWidth + columnOffset + 5) + "," + child.y + "," +
			xMax(parent.x * columnWidth + columnOffset) + "," + (child.y + 5) + ","
	else
		str += xMax(child.x * columnWidth + columnOffset) + "," + (child.y + 7) + "L"
	str += xMax(parent.x * columnWidth + columnOffset) + "," + (parent.y - 7)
	a.setAttribute("d", str);
	return maxX
}

this.renderLog = function(commits, yOffset=0){
	const text = commits.reduce((acc, cur, i) => {
		const index = i + yOffset;
		if(cur){
			return acc + `<div class="${index % 2 === 0 ? 'light' : 'dark'}"
		style="position: absolute; top:${index * rowHeight - rowHeight / 2 + rowOffset}px; width: 100%; height: ${rowHeight}px">
		<span class="valign" id="${cur.hash}">${
			cur.stat ? `
			<span class="insertions">+${cur.stat.insertions}</span>
			<span class="deletions">-${cur.stat.deletions}</span>`
			: ""
			}
			${cur.message}
		</span></div>`;
		}
		else{
			return acc;
		}
	}, "");
	$("#commits")[0].innerHTML += text;
};


/** Parse raw output from `git log --pretty=raw --numstat` and format for HTML
 */
this.parseLog = function(aCommits){
	allCommits = allCommits.concat(aCommits);

	// Cache hash id to object map for quick looking up
	for(var i = 0; i < aCommits.length; i++){
		commitMap[aCommits[i].hash] = aCommits[i]
	}

	// Cache children pointers from parents
	for(var i = 0; i < aCommits.length; i++){
		let commit = aCommits[i];
		let parents = commit.parents;
		for(var j = 0; j < parents.length; j++){
			var parent = commitMap[parents[j]]
			if(parent){
				parent.children = parent.children || []
				parent.children.push(commit)
			}
		}
		if(parents.length === 1){
			fetch(`/diff_stats/${parents[0]}/${commit.hash}`)
			.then((result) => result.json())
			.then((json) => {
				commit.stat = {
					insertions: json[0],
					deletions: json[1],
				};
				renderDiffStat(commit);
			})
		}
	}
}

/** Parse raw output from `git show-ref` command and save the
 * information internally for use with updateSvg.
 * 
 * @param {string} text
 */
this.parseRefs = function(aRefs){
	refs = {}
	for(const refPair of aRefs){
		refs[refPair[0]] = refPair[1];
	}
}

function findCommit(hash){
	if(hash.length < 4)
		throw "Hash length shorter than 4"
	for(var i = 0; i < allCommits.length; i++){
		if(allCommits[i].hash.substr(0, hash.length) === hash)
			return allCommits[i]
	}
	return null
}

function renderDiffStat(commit){
	if(!commit.stat)
		return;
	const rad = commit.stat ? 6 : 7;
	var c = circle(0, 0, rad, '#afafaf', '#000', "5");
	commit.svgGroup.appendChild(c)
	var addAngle = Math.min(Math.PI, (Math.log10(commit.stat.insertions + 1) + 0) * Math.PI / 5)
	var addArc = arc(0, 0, rad, 0, addAngle, 'green')
	commit.svgGroup.appendChild(addArc)
	var delAngle = -Math.min(Math.PI, (Math.log10(commit.stat.deletions + 1) + 0) * Math.PI / 5)
	var delArc = arc(0, 0, rad, delAngle, 0, 'red')
	commit.svgGroup.appendChild(delArc)

	const messageElem = $(`#${commit.hash}`)[0];
	if(messageElem){
		const deletionsElem = document.createElement("span");
		deletionsElem.className = "deletions";
		deletionsElem.innerHTML = `-${commit.stat.deletions}`;
		messageElem.prepend(deletionsElem);
		const insertionsElem = document.createElement("span");
		insertionsElem.className = "insertions";
		insertionsElem.innerHTML = `+${commit.stat.insertions} `;
		messageElem.prepend(insertionsElem);
	}
}

this.updateRefs = function(){
	// Update commit objects to have list of associated refs.
	// This method should be faster than other way around.
	for(var ref in refs){
		var id = refs[ref]
		if(id in commitMap){
			commitMap[id].refs = commitMap[id].refs || []
			commitMap[id].refs.push(ref)
		}
	}
};

let bgGroup;
let columns = [];
const colors = ['#7f0000', '#007f00', '#0000af', '#000000',
	'#7f7f00', '#7f007f', '#007f7f'];

this.updateSvg = function(svg, commentElem, commits=undefined, yOffset=0){
	var width = parseInt(svg.style.width);
	var height = parseInt(svg.style.height);

	commits = commits || allCommits;

	for(var i = 0; i < commits.length; i++){
		if(!commits[i].x){
			var commit = commits[i]

			// Find vacant column
			for(var j = 0; j < columns.length; j++){
				if(columns[j] === commit.hash){
					columns[j] = null
					break
				}
			}
			commit.x = j

			// Reserve columns for parents from vacant ones
			var numParents = commit.parents ? commit.parents.length : 0
			for(var k = 0; k < numParents; k++){
				const parentHash = commit.parents[k];
				for(var j = 0; j < columns.length; j++){
					if(!columns[j] || columns[j] === parentHash){
						break
					}
				}
				columns[j] = parentHash
			}
		}
		commits[i].y = (i + yOffset) * rowHeight + rowOffset
	}

	if(!bgGroup){
		bgGroup = document.createElementNS(NS,"g")
		svg.appendChild(bgGroup)
	}

	var colorIdx = 0
	for(var i = 0; i < commits.length; i++){
		var commit = commits[i]
		var rad = commit.stat ? 6 : 7
		var maxX = 0

		for(var j = 0; j < commit.parents.length; j++){
			var parent = findCommit(commit.parents[j])
			if(!parent)
				continue
			var parenti = commits.indexOf(parent)
			if(parenti < 0)
				continue
			var a = document.createElementNS(NS,"path");
			var x = setArrow(a, commit, parent);
			a.style.stroke = colors[colorIdx];
			// Try to keep the same color as long as the history is linear.
			// Otherwise, cycle colors.
			if(j !== 0 || parent.x !== commit.x)
				colorIdx = (colorIdx + 1) % colors.length
			a.style.strokeWidth = "2"
			a.style.fill = "none";
			a.style.pointerEvents = "none";
			svg.appendChild(a)

			if(maxX < x)
				maxX = x
		}

		// Add the commit marker circle after the connection lines, to make sure
		// the marker is painted on top of the lines.
		let group = document.createElementNS(NS,"g");
		var c = circle(0, 0, rad, '#afafaf', '#000', commit.stat ? "5" : "1")
		group.appendChild(c);
		group.setAttribute("transform", `translate(${commit.x * columnWidth + columnOffset} ${commit.y})`);
		svg.appendChild(group);
		commit.svgGroup = group;

		// Show refs
		var numRefs = commit.refs ? commit.refs.length : 0
		var refx = maxX + columnWidth
		for(var j = 0; j < numRefs; j++){
			var ref = commit.refs[j]
			var color = 0 <= ref.search(/^refs\/heads\//) ? '#00ff00' :
				0 <= ref.search(/^refs\/remotes\//) ? '#ffaf7f' :
				0 <= ref.search(/^refs\/tags\//) ? '#ffff00' : '#7f7f7f'
			// Truncate redundant prefixes
			var text = ref
				.replace(/^refs\/heads\//, '')
				.replace(/^refs\/remotes\//, '')
				.replace(/^refs\/tags\//, '')
			var refGroup = document.createElementNS(NS,"g")
			refGroup.setAttribute("transform", "translate(" + (refx)
				+ "," + (commit.y - rowHeight / 2) + ")");

			var t = document.createElementNS(NS,"text");
			t.setAttribute("x", "5");
			t.setAttribute("y", "15");
			t.setAttribute("class", "noselect");
			t.style.fontSize = "12px";
			t.style.fontFamily = "sans-serif";
			t.style.pointerEvents = "none";
			t.textContent = text;

			refGroup.appendChild(t)
			svg.appendChild(refGroup)

			// We can't measure width of text element until actually adding it
			// to the SVG, so we need to create surrounding box later and
			// insert before the text.
			var r = rect(0, 0, t.getBBox().width + 10, rowHeight, color)
			refGroup.insertBefore(r, t)

			refx += t.getBBox().width + 15
		}

		if(svg.getBoundingClientRect().width < refx)
			svg.style.width = Math.ceil(refx) + 'px'
	}

	lastCommits = 0 < commits.length ? [commits[commits.length-1]] : [];

	// Recalculate width by SVG content
	width = Math.ceil(svg.getBoundingClientRect().width)

	for(var i = 0; i < commits.length; i++){
		const index = i + yOffset;
		var bg = document.createElementNS(NS,"rect")
		bg.setAttribute('x', 0)
		bg.setAttribute('y', index * rowHeight - rowHeight / 2 + rowOffset)
		bg.setAttribute('width', width)
		bg.setAttribute('height', rowHeight)
		bg.setAttribute('class', index % 2 === 0 ? 'lightFill' : 'darkFill')
		bgGroup.appendChild(bg)
	}

	svg.style.height = ((allCommits.length) * rowHeight + rowOffset) + 'px'

	if(commentElem)
		commentElem.style.left = width + 'px'
}

let pendingFetch = false;

$(window).scroll((event) => {
	const scrollBottom = $(window).scrollTop() + document.documentElement.clientHeight;
	console.log(`scrollBottom ${scrollBottom}/${document.body.scrollHeight}`);
	if(document.body.scrollHeight <= scrollBottom){
		if(!pendingFetch && lastCommits.length !== 0){
			pendingFetch = true;
			console.log(`Pending fetch for ${lastCommits[0]} started`);
			fetch(`/commits/${lastCommits[0].hash}`)
			.then((resp) => resp.json())
			.then(json => {
				const commits = json;
				commits.shift();
				const yOffset = allCommits.length;
				this.renderLog(json, yOffset);
				this.parseLog(json);
				this.updateSvg($("#graph")[0], $('#commits')[0], json, yOffset);
				pendingFetch = false;
				console.log(`Pending fetch for ${lastCommits[0]} ended`);
			});
		}
	}
})


})()
