import { describe, expect, it } from 'vitest';
import { calculateBottomObstructionInset, calculateEffectiveBottomInset } from './viewport-insets';

describe('calculateBottomObstructionInset', () => {
    it('returns 0 when viewport fully visible', () => {
        expect(calculateBottomObstructionInset(800, 800, 0)).toBe(0);
    });

    it('returns obscured pixels when bottom browser controls consume height', () => {
        expect(calculateBottomObstructionInset(800, 744, 0)).toBe(56);
    });

    it('includes visual viewport offset when shifted from top', () => {
        expect(calculateBottomObstructionInset(800, 700, 20)).toBe(80);
    });

    it('clamps negative values to zero', () => {
        expect(calculateBottomObstructionInset(700, 720, 0)).toBe(0);
    });
});

describe('calculateEffectiveBottomInset', () => {
    it('keeps measured obstruction when it is larger than fallback', () => {
        expect(calculateEffectiveBottomInset({
            layoutViewportHeight: 900,
            visualViewportHeight: 760,
            visualViewportOffsetTop: 0,
            isAndroid: true,
            isFullscreen: false
        })).toBe(140);
    });

    it('uses Android fallback when obstruction appears as zero', () => {
        expect(calculateEffectiveBottomInset({
            layoutViewportHeight: 800,
            visualViewportHeight: 800,
            visualViewportOffsetTop: 0,
            isAndroid: true,
            isFullscreen: false
        })).toBe(56);
    });

    it('does not apply fallback on non-Android', () => {
        expect(calculateEffectiveBottomInset({
            layoutViewportHeight: 800,
            visualViewportHeight: 800,
            visualViewportOffsetTop: 0,
            isAndroid: false,
            isFullscreen: false
        })).toBe(0);
    });

    it('does not force Android fallback in fullscreen', () => {
        expect(calculateEffectiveBottomInset({
            layoutViewportHeight: 800,
            visualViewportHeight: 800,
            visualViewportOffsetTop: 0,
            isAndroid: true,
            isFullscreen: true
        })).toBe(0);
    });
});
