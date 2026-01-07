import { describe, it, expect } from 'vitest';
import { animationData, resolveAnimationId } from './animations';
import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';

describe('animation maps', () => {
    it('resolveAnimationId should resolve expected exercises', () => {
        expect(resolveAnimationId('Jumping Jacks')).toBe(AnimationId.JumpingJacks);
        // "Burpees" in json is "burpees", workout is "Burpees"
        expect(resolveAnimationId('Burpees')).toBe(AnimationId.Burpees);
        // "Ab Crunch" -> "AbCrunch"
        expect(resolveAnimationId('Ab Crunch')).toBe(AnimationId.AbCrunch);
    });

    it('animationData should have URL for IDs', () => {
        const id = AnimationId.PushUps;
        const url = animationData[id];
        expect(typeof url).toBe('string');
        expect(url.length).toBeGreaterThan(0);
    });
});
