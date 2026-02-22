import { describe, expect, it } from 'vitest';
import { calculateBottomObstructionInset } from './viewport-insets';

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
