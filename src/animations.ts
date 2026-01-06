import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';

// Dynamically import all animation binary files as URLs
const animationModules = import.meta.glob('./assets/animations/*.anim', {
    query: '?url',
    import: 'default',
    eager: true
}) as Record<string, string>;

export const animationData: Record<number, string> = {}; // Stores URL to .anim file
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
Object.entries(animationModules).forEach(([path, url]) => {
    const filename = path.split('/').pop()?.replace('.anim', '') || '';
    const id = FILENAME_TO_ID[filename];

    if (id !== undefined) {
        animationData[id] = url;
        DISPLAY_NAMES[id] = toDisplayName(filename);
    } else {
        console.warn(`Animation file ${filename}.anim does not match any AnimationId variant.`);
    }
});

/**
 * Fetch a binary animation file and return as Uint8Array
 */
export async function fetchAnimationBuffer(url: string): Promise<Uint8Array> {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Failed to fetch animation: ${url}`);
    }
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
}


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
    return AnimationId.Placeholder;
}

console.log(`Loaded ${Object.keys(animationData).length} animation files into library.`);
