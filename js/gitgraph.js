window.gitgraph = new (function(){
'use strict'

// Polyfill almost only for IE
Math.log10 = Math.log10 || function(x){
	return Math.log(x) / Math.LN10
}

var NS="http://www.w3.org/2000/svg";

var commitMap = {}
var commits = []

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
	var maxX = child.x * 20 + 30
	// Accumulate maximum x value
	function xMax(x){
		if(maxX < x) maxX = x
		return x
	}
	if(child.x < parent.x)
		str += xMax(child.x * 20 + 20 + 7) + "," + (child.y) + "L" +
			xMax(parent.x * 20 + 20 - 5) + "," + child.y + "," +
			xMax(parent.x * 20 + 20) + "," + (child.y + 5) + ","
	else if(parent.x < child.x)
		str += xMax(child.x * 20 + 20 - 7) + "," + (child.y) + "L" +
			xMax(parent.x * 20 + 20 + 5) + "," + child.y + "," +
			xMax(parent.x * 20 + 20) + "," + (child.y + 5) + ","
	else
		str += xMax(child.x * 20 + 20) + "," + (child.y + 7) + "L"
	str += xMax(parent.x * 20 + 20) + "," + (parent.y - 7)
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
		commitsElem.innerHTML += '<div style="position: absolute; left: 200px; top:' + (i * 20 + 13)
			+ 'px">' + commitStr + '</div>'
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

function findCommit(hash){
	if(hash.length < 4)
		throw "Hash length shorter than 4"
	for(var i = 0; i < commits.length; i++){
		if(commits[i].hash.substr(0, hash.length) === hash)
			return commits[i]
	}
	return null
}

this.updateSvg = function(svg){
	var width = parseInt(svg.style.width);
	var height = parseInt(svg.style.height);

	var columns = []

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
		commits[i].y = i * 20 + 20
	}

	for(var i = 0; i < commits.length; i++){
		var commit = commits[i]
		var rad = commit.stat ? 6 : 7
		var c = circle(commit.x * 20 + 20, commit.y, rad, '#afafaf', '#000', commit.stat ? "5" : "1")
		var maxX = 0
		svg.appendChild(c)

		if(commit.stat){
			var addAngle = Math.min(Math.PI, (Math.log10(commit.stat.add + 1) + 0) * Math.PI / 5)
			var addArc = arc(commit.x * 20 + 20, commit.y, rad, 0, addAngle, 'green')
			svg.appendChild(addArc)
			var delAngle = -Math.min(Math.PI, (Math.log10(commit.stat.del + 1) + 0) * Math.PI / 5)
			var delArc = arc(commit.x * 20 + 20, commit.y, rad, delAngle, 0, 'red')
			svg.appendChild(delArc)
		}

		for(var j = 0; j < commit.parents.length; j++){
			var parent = findCommit(commit.parents[j])
			if(!parent)
				continue
			var parenti = commits.indexOf(parent)
			if(parenti < 0)
				continue
			var a = document.createElementNS(NS,"path");
			var x = setArrow(a, commit, parent);
			a.style.stroke = "black";
			a.style.fill = "none";
			a.style.pointerEvents = "none";
			svg.appendChild(a)

			if(maxX < x)
				maxX = x
		}
		var a = document.createElementNS(NS,"path")
		a.setAttribute("d", "M" + (maxX + 10) + "," + commit.y + "L" + width + "," + commit.y)
		a.style.stroke = "#7f7f7f";
		a.style.fill = "none";
		a.style.pointerEvents = "none";
		svg.appendChild(a)
	}
	
	svg.style.height = (commits.length * 20 + 40) + 'px'
}


})()
