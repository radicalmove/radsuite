import App from "./App.svelte";
import { mount } from "svelte";
import "./styles.css";

const target = document.getElementById("root");

if (!target) {
  throw new Error("RADsuite root element was not found");
}

const app = mount(App, { target });

export default app;
