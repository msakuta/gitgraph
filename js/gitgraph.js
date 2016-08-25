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
var commits = []
var refs = {}

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


/** Parse raw output from `git log --pretty=raw --numstat` and format for HTML
 */
this.parseLog = function(text, commitsElem){
	var commitStrs = text.match(/^commit [0-9a-f]+\r?\n(.|\r|\n)+?(?=^commit [0-9a-f]+)/mg)
	if(!commitStrs)
		return
	for(var i = 0; i < commitStrs.length; i++){
		var str = commitStrs[i]
		var commitStr = str.match(/^commit [0-9a-f]+/)[0]
		var commitHash = commitStr.substr("commit ".length).trim()
		commitStr = commitHash.substr(0,6)
		var parentMatch = str.match(/^parent [0-9a-f]+/gm)
		var commitObj = {
			hash: commitHash,
			msg: str.match(/^    .+/gm)
		}
		if(parentMatch){
			commitObj.parents = []
			for(var j = 0; j < parentMatch.length; j++){
				commitObj.parents.push(parentMatch[j].substr("parent ".length).trim())
				commitStr += ' ' + parentMatch[j].substr("parent ".length, 6).trim()
			}
		}

		// Check added/deleted lines
		var statMatch = str.match(/^\d+\t\d+\t.+/gm)
		if(statMatch){
			commitObj.stat = {add: 0, del: 0, files: []}
			for(var j = 0; j < statMatch.length; j++){
				var re = /^(\d+)\t(\d+)\t(.+)/.exec(statMatch[j])
				// Ignore binary files for now
				if(re && re[1] !== '-' && re[2] !== '-'){
					commitObj.stat.add += parseInt(re[1])
					commitObj.stat.del += parseInt(re[2])
					commitObj.stat.files.push({add: re[1], del: re[2], file: re[3]})
				}
			}
			commitStr += ' <span style="color:green">+' + commitObj.stat.add + '</span> ' +
				'<span style="color:red">-' + commitObj.stat.del + '</span>'
		}

		if(commitObj.msg && 0 < commitObj.msg.length)
			commitStr += commitObj.msg[0]
		commits.push(commitObj)
		commitsElem.innerHTML += '<div class="' + (i % 2 === 0 ? 'light' : 'dark') +
			'" style="position: absolute; top:' + (i * rowHeight - rowHeight / 2 + rowOffset) +
			'px; width: 100%; height: ' + rowHeight + 'px"><span class="valign">' +
			commitStr + '</span></div>'
	}

	// Cache hash id to object map for quick looking up
	for(var i = 0; i < commits.length; i++){
		commitMap[commits[i].hash] = commits[i]
	}

	// Cache children pointers from parents
	for(var i = 0; i < commits.length; i++){
		var parents = commits[i].parents
		for(var j = 0; j < parents.length; j++){
			var parent = commitMap[parents[j]]
			if(parent){
				parent.children = parent.children || []
				parent.children.push(commits[i])
			}
		}
	}
}

/** Parse raw output from `git show-ref` command and save the
 * information internally for use with updateSvg.
 * 
 * @param {string} text
 */
this.parseRefs = function(text){
	var refStrs = text.match(/^[0-9a-f]+ .+$/mg)
	if(!refStrs)
		return
	for(var i = 0; i < refStrs.length; i++){
		var refStr = refStrs[i]
		var re = /^([0-9a-f]+) (.+)/.exec(refStr)
		// Insert into the map with reference name as the key.
		if(re && re[1] !== '' && re[2] !== ''){
			refs[re[2]] = re[1]
		}
	}
}

function findCommit(hash){
	if(hash.length < 4)
		throw "Hash length shorter than 4"
	for(var i = 0; i < commits.length; i++){
		if(commits[i].hash.substr(0, hash.length) === hash)
			return commits[i]
	}
	return null
}

this.updateSvg = function(svg, commentElem){
	var width = parseInt(svg.style.width);
	var height = parseInt(svg.style.height);

	var columns = []
	var colors = ['#7f0000', '#007f00', '#0000af', '#000000',
		'#7f7f00', '#7f007f', '#007f7f']

	// Update commit objects to have list of associated refs.
	// This method should be faster than other way around.
	for(var ref in refs){
		var id = refs[ref]
		if(id in commitMap){
			commitMap[id].refs = commitMap[id].refs || []
			commitMap[id].refs.push(ref)
		}
	}

	for(var i = 0; i < commits.length; i++){
		if(!commits[i].x){
			var commit = commits[i]

			// Find vacant column
			for(var j = 0; j < columns.length; j++){
				if(columns[j] === commit){
					columns[j] = null
					break
				}
			}
			commit.x = j

			// Reserve columns for parents from vacant ones
			var numParents = commit.parents ? commit.parents.length : 0
			for(var k = 0; k < numParents; k++){
				var parent = commitMap[commit.parents[k]]
				for(var j = 0; j < columns.length; j++){
					if(!columns[j] || columns[j] === parent){
						break
					}
				}
				columns[j] = parent
			}
		}
		commits[i].y = i * rowHeight + rowOffset
	}

	var bgGroup = document.createElementNS(NS,"g")
	svg.appendChild(bgGroup)

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
		var c = circle(commit.x * columnWidth + columnOffset, commit.y, rad, '#afafaf', '#000', commit.stat ? "5" : "1")
		svg.appendChild(c)
		if(commit.stat){
			var addAngle = Math.min(Math.PI, (Math.log10(commit.stat.add + 1) + 0) * Math.PI / 5)
			var addArc = arc(commit.x * columnWidth + columnOffset, commit.y, rad, 0, addAngle, 'green')
			svg.appendChild(addArc)
			var delAngle = -Math.min(Math.PI, (Math.log10(commit.stat.del + 1) + 0) * Math.PI / 5)
			var delArc = arc(commit.x * columnWidth + columnOffset, commit.y, rad, delAngle, 0, 'red')
			svg.appendChild(delArc)
		}

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

	// Recalculate width by SVG content
	width = Math.ceil(svg.getBoundingClientRect().width)

	for(var i = 0; i < commits.length; i++){
		var bg = document.createElementNS(NS,"rect")
		bg.setAttribute('x', 0)
		bg.setAttribute('y', i * rowHeight - rowHeight / 2 + rowOffset)
		bg.setAttribute('width', width)
		bg.setAttribute('height', rowHeight)
		bg.setAttribute('class', i % 2 === 0 ? 'lightFill' : 'darkFill')
		bgGroup.appendChild(bg)
	}

	svg.style.height = ((commits.length) * rowHeight + rowOffset) + 'px'

	if(commentElem)
		commentElem.style.left = width + 'px'
}


})()
