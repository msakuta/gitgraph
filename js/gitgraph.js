window.gitgraph = new (function(){
'use strict'

var NS="http://www.w3.org/2000/svg";

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

function setArrow(a,x0,y0,x1,y1){
	a.setAttribute("d", "M" + x0 + "," + y0 + "L" + x1 + "," + y1 + "," + (x1 - 5) + "," + (y1 - 5) + "M" + x1 + "," + y1 + "," + (x1 - 5) + "," + (y1 + 5));
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

	for(var i = 0; i < commits.length; i++){
		if(i === 0)
			commits[i].x = 20
		else if(commits[i].parents && 2 <= commits[i].parents.length)
			commits[i].x = (commits[i-1].x + 20) % (width - 40)
		else
			commits[i].x = commits[i-1].x
	}

	for(var i = 0; i < commits.length; i++){
		var commit = commits[i]
		var c = circle(commit.x, i * 20 + 20, 7, '#7f7f7f')
		svg.appendChild(c)

		var t = document.createElementNS(NS,"text");
		//t.y.baseVal.value = 120;
		t.setAttribute("x", commit.x + 12);
		t.setAttribute("y", i * 20 + 20 + 3);
		// Need a CSS with class "noselect" which specify selection disabling style
		t.setAttribute("class", "noselect");
		t.style.fontSize = "12px";
		t.style.fontFamily = "monospace";
		t.style.pointerEvents = "none";
		t.textContent = commit.hash.substr(0,6);
		svg.appendChild(t);

		if(commit.parents){
			for(var j = 0; j < commit.parents.length; j++){
				var parent = findCommit(commit.parents[j])
				if(!parent)
					continue
				var parenti = commits.indexOf(parent)
				if(parenti < 0)
					continue
				var a = document.createElementNS(NS,"path");
				setArrow(a, commit.x, i * 20 + 20 + 7,
					parent.x, parenti * 20 + 20 - 7);
				a.style.stroke = "black";
				a.style.fill = "none";
				a.style.pointerEvents = "none";
				svg.appendChild(a)
			}
		}
	}
	
	svg.style.height = (commits.length * 20 + 40) + 'px'
}


})()
