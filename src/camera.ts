/**
 * Orbit Camera Controller
 *
 * Thin input handler that forwards keyboard/mouse input to Rust/WASM
 * for quaternion-based camera rotation. All quaternion math is done in Rust.
 */

import { rotate_camera, sync_camera, get_camera_right_axis, get_current_view_matrix, get_current_projection_matrix } from '../wasm/pkg/jokkerin_ventti_wasm';


// Input state
let isDragging = false;
let lastMouseX = 0;
let lastMouseY = 0;

// Camera enabled state (disabled when editing joints)
let cameraEnabled = true;

// Sensitivity settings
const KEYBOARD_SPEED = 0.03;     // Radians per frame when key held
const MOUSE_SENSITIVITY = 0.005; // Radians per pixel dragged

// Track which keys are pressed
const keysPressed: Set<string> = new Set();

/**
 * Enable or disable camera controls
 */
export function setCameraEnabled(enabled: boolean): void {
    cameraEnabled = enabled;
    if (!enabled) {
        keysPressed.clear();
        isDragging = false;
    }
}

/**
 * Get camera enabled state
 */
export function getCameraState() {
    return { enabled: cameraEnabled };
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

    // Apply initial camera position (state initialized in Rust)
    sync_camera();
}

/**
 * Update camera based on currently pressed keys
 * Call this every frame
 */
export function updateCameraFromInput(): void {
    let changed = false;

    // Horizontal rotation (around world Y axis)
    if (keysPressed.has('ArrowLeft')) {
        rotate_camera(0, 1, 0, -KEYBOARD_SPEED);
        changed = true;
    }
    if (keysPressed.has('ArrowRight')) {
        rotate_camera(0, 1, 0, KEYBOARD_SPEED);
        changed = true;
    }

    // Vertical rotation (around camera's local right axis)
    if (keysPressed.has('ArrowUp') || keysPressed.has('ArrowDown')) {
        const right = get_camera_right_axis();
        const dir = keysPressed.has('ArrowUp') ? -1 : 1;  // Inverted: Up = negative rotation
        rotate_camera(right[0], right[1], right[2], dir * KEYBOARD_SPEED);
        changed = true;
    }

    if (changed) {
        sync_camera();
    }
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

    // Horizontal drag: rotate around world Y axis
    if (Math.abs(deltaX) > 0) {
        rotate_camera(0, 1, 0, deltaX * MOUSE_SENSITIVITY);
    }

    // Vertical drag: rotate around world X axis
    if (Math.abs(deltaY) > 0) {
        rotate_camera(1, 0, 0, deltaY * MOUSE_SENSITIVITY);
    }

    lastMouseX = e.clientX;
    lastMouseY = e.clientY;

    sync_camera();
}

function onPointerUp(e: PointerEvent): void {
    isDragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
}

/**
 * Get view matrix as Float32Array (for handle-based API)
 */
export function getViewMatrix(): Float32Array {
    const arr = get_current_view_matrix();
    return new Float32Array(arr);
}

/**
 * Get projection matrix as Float32Array (for handle-based API)
 */
export function getProjectionMatrix(): Float32Array {
    const arr = get_current_projection_matrix();
    return new Float32Array(arr);
}
