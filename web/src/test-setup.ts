// Tests run in English whatever the machine's language: they check English
// text and formats unless they switch the locale themselves.
import { beforeEach } from 'vitest';
import { setLocale } from './lib/i18n/index.svelte';

beforeEach(() => setLocale('en'));
