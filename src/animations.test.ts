import { describe, it, expect } from 'vitest';
import { animationMap } from './animations';

describe('animationMap', () => {
    it('should be a Map', () => {
        expect(animationMap).toBeInstanceOf(Map);
    });

    it('should contain expected keys', () => {
        expect(animationMap.has('Jumping Jacks')).toBe(true);
        expect(animationMap.has('Burpees')).toBe(true);
    });
    
    it('should have string values (JSON content)', () => {
        const jumpingJacks = animationMap.get('Jumping Jacks');
        expect(typeof jumpingJacks).toBe('string');
        // Basic check if it looks like JSON
        expect(jumpingJacks?.trim().startsWith('{')).toBe(true);
    });
});
