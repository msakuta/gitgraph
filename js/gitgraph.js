window.gitgraph = new (function(){
'use strict'

/** Parse raw output from `git log --stat --raw` and format for HTML
 */
this.parseLog = function(text, commitsElem){
	var commits = text.match(/^commit [0-9a-f]+/mg)
	if(!commits)
		return
	for(var i = 0; i < commits.length; i++)
		commitsElem.innerHTML += commits[i] + '\n'
}

})()
