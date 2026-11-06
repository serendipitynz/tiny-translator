import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("index.html has no #app element to mount into");
}

const app = mount(App, { target });

export default app;
