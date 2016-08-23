window.gitgraph = new (function(){
'use strict'

/** Parse raw output from `git log --stat --raw` and format for HTML
 */
this.parseLog = function(text, commitsElem){
	var commits = text.match(/^commit [0-9a-f]+(\nMerge:( [0-9a-f]+)+)?/mg)
	if(!commits)
		return
	for(var i = 0; i < commits.length; i++){
		var str = commits[i]
		var commitStr = str.match(/^commit [0-9a-f]+/)
		var mergeStr = str.match(/Merge:( [0-9a-f]+)+/)
		if(mergeStr)
			commitStr += '  ' + mergeStr[0]
		commitsElem.innerHTML += commitStr + '\n'
	}
}

})()
