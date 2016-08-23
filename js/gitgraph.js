window.gitgraph = new (function(){
'use strict'

var NS="http://www.w3.org/2000/svg";

var commitMap = {}
var commits = []

function circle(cx,cy,r,fill,stroke){
	var c = document.createElementNS(NS,"circle");
	c.cx.baseVal.value = cx;
	c.cy.baseVal.value = cy;
	c.r.baseVal.value = r;
	c.style.stroke = stroke || "black";
	c.style.strokeWidth = "1";
	c.style.fill = fill || "white";
	return c;
}

function setArrow(a,child,parent){
	var str = "M"
	if(child.x < parent.x)
		str += (child.x * 20 + 20 + 7) + "," + (child.y) + "L" +
			(parent.x * 20 + 20 - 5) + "," + child.y + "," +
			(parent.x * 20 + 20) + "," + (child.y + 5) + ","
	else if(parent.x < child.x)
		str += (child.x * 20 + 20 - 7) + "," + (child.y) + "L" +
			(parent.x * 20 + 20 + 5) + "," + child.y + "," +
			(parent.x * 20 + 20) + "," + (child.y + 5) + ","
	else
		str += (child.x * 20 + 20) + "," + (child.y + 7) + "L"
	str += (parent.x * 20 + 20) + "," + (parent.y - 7)
	a.setAttribute("d", str);
}


/** Parse raw output from `git log --pretty=raw` and format for HTML
 */
this.parseLog = function(text, commitsElem){
	var commitStrs = text.match(/^commit [0-9a-f]+\r?\ntree [0-9a-f]+(\r?\nparent [0-9a-f]+)*/mg)
	if(!commitStrs)
		return
	for(var i = 0; i < commitStrs.length; i++){
		var str = commitStrs[i]
		var commitStr = str.match(/^commit [0-9a-f]+/)[0]
		var parentMatch = str.match(/parent [0-9a-f]+/g)
		var commitObj = {
			hash: commitStr.substr("commit ".length).trim()
		}
		if(parentMatch){
			commitObj.parents = []
			for(var j = 0; j < parentMatch.length; j++){
				commitObj.parents.push(parentMatch[j].substr("parent ".length).trim())
				commitStr += ' ' + parentMatch[j].substr("parent ".length, 6).trim()
			}
		}
		commits.push(commitObj)
		commitsElem.innerHTML += commitStr + '\n'
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

			// Clear children
			var numChildren = commit.children ? commit.children.length : 0
			for(var k = 0; k < numChildren; k++){
				for(var j = 0; j < columns.length; j++){
					if(columns[j] && columns[j] === commit.children[k]){
						columns[j] = null
						break
					}
				}
			}

			// Find vacant column 
			for(var j = 0; j < columns.length; j++){
				if(!columns[j]){
					break
				}
			}
			commit.x = j
			columns[j] = commit
		}
		commits[i].y = i * 20 + 20
	}

	for(var i = 0; i < commits.length; i++){
		var commit = commits[i]
		var c = circle(commit.x * 20 + 20, commit.y, 7, '#7f7f7f')
		svg.appendChild(c)

		var t = document.createElementNS(NS,"text");
		//t.y.baseVal.value = 120;
		t.setAttribute("x", commit.x * 20 + 20 + 12);
		t.setAttribute("y", commit.y);
		// Need a CSS with class "noselect" which specify selection disabling style
		t.setAttribute("class", "noselect");
		t.style.fontSize = "12px";
		t.style.fontFamily = "monospace";
		t.style.pointerEvents = "none";
		t.textContent = commit.hash.substr(0,6);
		svg.appendChild(t);

		for(var j = 0; j < commit.parents.length; j++){
			var parent = findCommit(commit.parents[j])
			if(!parent)
				continue
			var parenti = commits.indexOf(parent)
			if(parenti < 0)
				continue
			var a = document.createElementNS(NS,"path");
			setArrow(a, commit, parent);
			a.style.stroke = "black";
			a.style.fill = "none";
			a.style.pointerEvents = "none";
			svg.appendChild(a)
		}
	}
	
	svg.style.height = (commits.length * 20 + 40) + 'px'
}


})()
