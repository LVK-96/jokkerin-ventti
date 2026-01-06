import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';

// Dynamically import all animation JSONs as raw strings
const animationModules = import.meta.glob('./assets/animations/*.json', {
    query: '?raw',
    import: 'default',
    eager: true
}) as Record<string, string>;

export const animationData: Record<number, string> = {};
export const DISPLAY_NAMES: Record<number, string> = {};

// Helper: "AbCrunch" -> "ab_crunch"
function toSnakeCase(str: string): string {
    return str.replace(/[A-Z]/g, (letter, index) => {
        return (index === 0 ? '' : '_') + letter.toLowerCase();
    }).replace(/^_/, '');
}

// Helper: "ab_crunch" -> "Ab Crunch"
function toDisplayName(snake: string): string {
    return snake
        .split(/[_-]/)
        .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(' ');
}

// Build mapping of snake_case filename -> AnimationId
const FILENAME_TO_ID: Record<string, number> = {};
Object.entries(AnimationId).forEach(([key, val]) => {
    if (typeof val === 'number') {
        const snake = toSnakeCase(key);
        FILENAME_TO_ID[snake] = val;
    }
});

// Populate animation data
Object.entries(animationModules).forEach(([path, content]) => {
    const filename = path.split('/').pop()?.replace('.json', '') || '';
    const id = FILENAME_TO_ID[filename];

    if (id !== undefined) {
        animationData[id] = content;
        DISPLAY_NAMES[id] = toDisplayName(filename);
    } else {
        console.warn(`Animation file ${filename}.json does not match any AnimationId variant.`);
    }
});

/**
 * Resolve an exercise name to an AnimationId.
 * Fallback to placeholder if not found.
 */
export function resolveAnimationId(name: string): AnimationId {
    // 1. Try to find match in enum by converting to PascalCase
    const pascal = name.replace(/[^a-zA-Z0-9]/g, ' ')
        .split(' ')
        .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
        .join('');

    if ((AnimationId as any)[pascal] !== undefined) {
        return (AnimationId as any)[pascal] as AnimationId;
    }

    // 2. Special cases / common variations
    const slug = name.toLowerCase().replace(/[^a-z0-9]/g, '');
    if (slug === 'pushups') return AnimationId.PushUps;

    // Default to placeholder
    return AnimationId.placeholder;
}

console.log(`Loaded ${Object.keys(animationData).length} animation files into library.`);
