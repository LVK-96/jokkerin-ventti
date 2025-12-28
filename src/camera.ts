/**
 * Orbit Camera Controller
 * 
 * Provides keyboard (arrow keys) and mouse drag controls for orbiting
 * the camera around the stickman.
 */

import { update_camera } from '../wasm/pkg/jokkerin_ventti_wasm';

// Camera state (spherical coordinates)
let azimuth = 0.7;      // Horizontal angle in radians (0 = front)
let elevation = 0.25;   // Vertical angle in radians (0 = level)
let distance = 4.0;     // Distance from target

// Input state
let isDragging = false;
let lastMouseX = 0;
let lastMouseY = 0;

// Camera enabled state (disabled when editing joints)
let cameraEnabled = true;

// Sensitivity settings
const KEYBOARD_SPEED = 0.03;    // Radians per frame when key held
const MOUSE_SENSITIVITY = 0.005; // Radians per pixel dragged

// Limits
const MIN_ELEVATION = 0.05;  // Prevent going below floor level
const MAX_ELEVATION = 1.4;   // Don't go directly overhead

// Track which keys are pressed
const keysPressed: Set<string> = new Set();

/**
 * Enable or disable camera controls
 */
export function setCameraEnabled(enabled: boolean): void {
    cameraEnabled = enabled;
    if (!enabled) {
        // Clear any held keys when disabling
        keysPressed.clear();
        isDragging = false;
    }
}

export function getCameraState() {
    return { azimuth, elevation };
}

/**
 * Initialize camera controls
 */
export function initCameraControls(canvas: HTMLCanvasElement): void {
    // Keyboard events (use document for global capture)
    document.addEventListener('keydown', onKeyDown);
    document.addEventListener('keyup', onKeyUp);

    // Mouse events on canvas
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('pointermove', onPointerMove);
    canvas.addEventListener('pointerup', onPointerUp);
    canvas.addEventListener('pointerleave', onPointerUp);

    // Apply initial camera position
    syncCamera();
}

/**
 * Update camera based on currently pressed keys
 * Call this every frame
 */
export function updateCameraFromInput(): void {
    let changed = false;

    if (keysPressed.has('ArrowLeft')) {
        azimuth -= KEYBOARD_SPEED;
        changed = true;
    }
    if (keysPressed.has('ArrowRight')) {
        azimuth += KEYBOARD_SPEED;
        changed = true;
    }
    if (keysPressed.has('ArrowUp')) {
        elevation = Math.min(MAX_ELEVATION, elevation + KEYBOARD_SPEED);
        changed = true;
    }
    if (keysPressed.has('ArrowDown')) {
        elevation = Math.max(MIN_ELEVATION, elevation - KEYBOARD_SPEED);
        changed = true;
    }

    if (changed) {
        syncCamera();
    }
}

/**
 * Send current camera state to WASM
 */
function syncCamera(): void {
    update_camera(azimuth, elevation, distance);
}

// --- Event Handlers ---

function onKeyDown(e: KeyboardEvent): void {
    if (!cameraEnabled) return;
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
        keysPressed.add(e.key);
        e.preventDefault();
    }
}

function onKeyUp(e: KeyboardEvent): void {
    keysPressed.delete(e.key);
}

function onPointerDown(e: PointerEvent): void {
    if (!cameraEnabled) return;
    isDragging = true;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent): void {
    if (!isDragging) return;

    const deltaX = e.clientX - lastMouseX;
    const deltaY = e.clientY - lastMouseY;

    azimuth += deltaX * MOUSE_SENSITIVITY;
    elevation = Math.max(MIN_ELEVATION, Math.min(MAX_ELEVATION,
        elevation + deltaY * MOUSE_SENSITIVITY));

    lastMouseX = e.clientX;
    lastMouseY = e.clientY;

    syncCamera();
}

function onPointerUp(e: PointerEvent): void {
    isDragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
}
