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
});

export function resolveAnimationId(name: string): AnimationId | undefined {
    return LOOKUP_MAP[name] as AnimationId | undefined;
}

// Logs for debugging
console.log(`Loaded ${Object.keys(animationData).length} animations.`);
