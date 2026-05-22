import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { init as l10nInit } from "./l10n";

l10nInit();

const app = mount(App, { target: document.body });

export default app;
