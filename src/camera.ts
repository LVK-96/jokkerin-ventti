/**
 * Orbit Camera Controller
 *
 * Thin input handler that forwards keyboard/mouse input to Rust/WASM
 * for quaternion-based camera rotation. All quaternion math is done in Rust.
 */

import type { App } from '../wasm/pkg/jokkerin_ventti_wasm';

// Current App instance - set during initialization
let app: App | null = null;

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
 * @param canvas The canvas element to attach controls to
 * @param wasmApp The WASM App instance for camera operations
 */
export function initCameraControls(canvas: HTMLCanvasElement, wasmApp: App): void {
    app = wasmApp;

    // Keyboard events (use document for global capture)
    document.addEventListener('keydown', onKeyDown);
    document.addEventListener('keyup', onKeyUp);

    // Mouse events on canvas
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('pointermove', onPointerMove);
    canvas.addEventListener('pointerup', onPointerUp);
    canvas.addEventListener('pointerleave', onPointerUp);
    canvas.addEventListener('pointercancel', (e) => {
        onPointerUp(e);
    });

    // Apply initial camera position (state initialized in Rust)
    app.sync_camera();
}

/**
 * Update camera based on currently pressed keys
 * Call this every frame
 */
export function updateCameraFromInput(): void {
    if (!app) return;

    let changed = false;

    // Horizontal rotation (around world Y axis)
    if (keysPressed.has('ArrowLeft')) {
        app.rotate_camera(0, 1, 0, -KEYBOARD_SPEED);
        changed = true;
    }
    if (keysPressed.has('ArrowRight')) {
        app.rotate_camera(0, 1, 0, KEYBOARD_SPEED);
        changed = true;
    }

    // Vertical rotation (around camera's local right axis)
    if (keysPressed.has('ArrowUp') || keysPressed.has('ArrowDown')) {
        const right = app.get_camera_right_axis();
        const dir = keysPressed.has('ArrowUp') ? -1 : 1;  // Inverted: Up = negative rotation
        app.rotate_camera(right[0], right[1], right[2], dir * KEYBOARD_SPEED);
        changed = true;
    }

    // Apply Joystick Velocity (Touch)
    if (rotationVelocityX !== 0) {
        app.rotate_camera(0, 1, 0, rotationVelocityX);
        changed = true;
    }
    if (rotationVelocityY !== 0) {
        app.rotate_camera(1, 0, 0, rotationVelocityY);
        changed = true;
    }

    if (changed) {
        app.sync_camera();
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

// Joystick state for touch
let touchStartX = 0;
let touchStartY = 0;
let rotationVelocityX = 0;
let rotationVelocityY = 0;

// Config
const JOYSTICK_SENSITIVITY = 0.0002;
const DEADZONE = 10;

function onPointerDown(e: PointerEvent): void {
    if (!cameraEnabled) return;

    isDragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);

    if (e.pointerType === 'touch') {
        touchStartX = e.clientX;
        touchStartY = e.clientY;
        rotationVelocityX = 0;
        rotationVelocityY = 0;
    } else {
        lastMouseX = e.clientX;
        lastMouseY = e.clientY;
    }
}

function onPointerMove(e: PointerEvent): void {
    if (!isDragging || !app) return;

    if (e.pointerType === 'touch') {
        // Joystick Logic
        const dx = e.clientX - touchStartX;
        const dy = e.clientY - touchStartY;

        rotationVelocityX = (Math.abs(dx) > DEADZONE)
            ? (dx - Math.sign(dx) * DEADZONE) * JOYSTICK_SENSITIVITY
            : 0;

        rotationVelocityY = (Math.abs(dy) > DEADZONE)
            ? (dy - Math.sign(dy) * DEADZONE) * JOYSTICK_SENSITIVITY
            : 0;
    } else {
        // Standard Mouse Drag (1:1)
        const deltaX = e.clientX - lastMouseX;
        const deltaY = e.clientY - lastMouseY;

        if (Math.abs(deltaX) > 0) {
            app.rotate_camera(0, 1, 0, deltaX * MOUSE_SENSITIVITY);
        }
        if (Math.abs(deltaY) > 0) {
            app.rotate_camera(1, 0, 0, deltaY * MOUSE_SENSITIVITY);
        }

        lastMouseX = e.clientX;
        lastMouseY = e.clientY;
        app.sync_camera();
    }
}

function onPointerUp(e: PointerEvent): void {
    isDragging = false;
    rotationVelocityX = 0;
    rotationVelocityY = 0;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
}

/**
 * Get view matrix as Float32Array (for handle-based API)
 */
export function getViewMatrix(): Float32Array {
    if (!app) return new Float32Array(16);
    const arr = app.get_current_view_matrix();
    return new Float32Array(arr);
}

/**
 * Get projection matrix as Float32Array (for handle-based API)
 */
export function getProjectionMatrix(): Float32Array {
    if (!app) return new Float32Array(16);
    const arr = app.get_current_projection_matrix();
    return new Float32Array(arr);
}
