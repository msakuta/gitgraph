import App from './App.svelte';

let svelte = new App({
    target: document.getElementById('graphApp'),
    props: {
        hello: ["Task 1", "Task 2"]
    }
});
