import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
import { applyTheme, loadTheme } from './lib/themes';

// The remembered theme, before the first paint.
applyTheme(loadTheme());

const target = document.getElementById('app');
if (!target) {
  throw new Error('missing #app element');
}

export default mount(App, { target });
