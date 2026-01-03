// Animation data - loaded as raw text for wasm parsing


import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';

// Dynamically import all animation JSONs as raw strings
const animationModules = import.meta.glob('./assets/animations/*.json', {
    query: '?raw',
    import: 'default',
    eager: true
}) as Record<string, string>;

export const animationData: Record<number, string> = {};
export const DISPLAY_NAMES: Record<number, string> = {};
export const LOOKUP_MAP: Record<string, number> = {};

// Helper: "ab_crunch" -> "Ab Crunch"
function toDisplayName(snake: string): string {
    return snake
        .split(/[_-]/) // split on underscore or dash
        .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(' ');
}

// Sort keys to ensure deterministic order matching Rust's fs::read_dir + sort
const sortedPaths = Object.keys(animationModules).sort();

sortedPaths.forEach((path, index) => {
    const content = animationModules[path];
    const id = index as AnimationId;

    // 1. Store Content
    animationData[id] = content;

    // 2. Derive Display Name from filename (e.g. "ab_crunch.json" -> "Ab Crunch")
    // This removes dependency on parsing JSON "name" field for metadata
    const filename = path.split('/').pop()?.replace('.json', '') || '';
    const displayName = toDisplayName(filename);

    DISPLAY_NAMES[id] = displayName;

    // 3. Build Lookup Map for Workout runner (case-insensitive)
    // Maps "Ab Crunch" -> ID, "ab crunch" -> ID, "ab_crunch" -> ID
    LOOKUP_MAP[displayName] = id;
    LOOKUP_MAP[displayName.toLowerCase()] = id;
    LOOKUP_MAP[filename] = id; // Also map exact filename just in case

    // 4. Robust Lookup (Slug)
    // Matches "Push-Ups" -> "pushups" and "Push Ups" -> "pushups"
    const slug = displayName.toLowerCase().replace(/[^a-z0-9]/g, '');
    LOOKUP_MAP[slug] = id;
});

export function resolveAnimationId(name: string): AnimationId | undefined {
    // Try exact match first
    if (LOOKUP_MAP[name] !== undefined) return LOOKUP_MAP[name] as AnimationId;

    // Try lower case
    if (LOOKUP_MAP[name.toLowerCase()] !== undefined) return LOOKUP_MAP[name.toLowerCase()] as AnimationId;

    // Try slug
    const slug = name.toLowerCase().replace(/[^a-z0-9]/g, '');
    return LOOKUP_MAP[slug] as AnimationId | undefined;
}

// Logs for debugging
console.log(`Loaded ${Object.keys(animationData).length} animations.`);
