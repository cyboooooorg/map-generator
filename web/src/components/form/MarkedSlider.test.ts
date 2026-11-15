import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import MarkedSlider, { type SliderMark } from './MarkedSlider.svelte';
import StarDot from './StarDot.svelte';

const mark = (label: string, fraction: number, over: Partial<SliderMark> = {}): SliderMark => ({ label, fraction, onpick: () => {}, ...over });
const html = (marks: SliderMark[]) =>
  render(MarkedSlider, { props: { min: 0, max: 1000, step: 1, value: 500, label: 'Mass', oninput: () => {}, marks, marksLabel: 'Reference masses' } }).body;

describe('MarkedSlider', () => {
  it('draws one tick per mark on the same rail as the range input, at the thumb-aware place', () => {
    const out = html([mark('Earth 1', 0.5, { earth: true }), mark('Mars 0.107', 0.2)]);
    expect(out.match(/<input type="range"/g)).toHaveLength(1);
    expect(out.match(/class="tick[ "]/g)).toHaveLength(2);
    expect(out).toContain('left: calc(8px + 0.5 * (100% - 16px))');
    expect(out).toContain('left: calc(8px + 0.2 * (100% - 16px))');
  });

  it('puts a chip under each tick that sets the value, with the group named', () => {
    const out = html([mark('Earth 1', 0.5, { note: '(5.97×10²¹ t)' })]);
    expect(out).toMatch(/role="group" aria-label="Reference masses"/);
    expect(out).toMatch(/<button type="button" class="chip[^"]*"[^>]*>Earth 1<!--\[0--><small[^>]*>\(5\.97×10²¹ t\)<\/small>/);
  });

  it('draws no ticks nor chips without marks', () => {
    const out = html([]);
    expect(out).not.toContain('tick');
    expect(out).not.toContain('chip');
  });
});

describe('StarDot', () => {
  it('fills the disc with the colour it is given', () => {
    const out = render(StarDot, { props: { colour: '#ff7a4d', label: 'M5V' } }).body;
    expect(out).toContain('aria-label="M5V"');
    expect(out).toContain('background: #ff7a4d');
    expect(out).not.toContain('empty');
  });
  it('is an empty ring when no colour is known', () => {
    const out = render(StarDot, { props: { colour: null, label: 'Auto' } }).body;
    expect(out).toContain('empty');
  });
});
