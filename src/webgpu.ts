/**
 * WebGPU + Wasm integration module
 *
 * Loads the Rust Wasm module and initializes WebGPU from Rust.
 */

import init, { init_gpu, render_frame, resize_gpu, update_time_uniform, update_skeleton, set_exercise, load_animation, add, log } from '../wasm/pkg/jokkerin_ventti_wasm';
import { initCameraControls, updateCameraFromInput } from './camera';

export { set_exercise, load_animation };

let animationId: number | null = null;
let lastTime = 0;
let initialized = false;

/**
 * Animation loop using requestAnimationFrame
 */
function animate(time: number): void {
    if (!initialized) {
        lastTime = time;
        initialized = true;
    }

    const delta = time - lastTime;
    lastTime = time;

    // Update time uniform in Rust
    update_time_uniform(delta);

    // Update camera from keyboard input
    updateCameraFromInput();

    // Update skeleton animation
    update_skeleton();

    // Render the frame
    render_frame();

    // Schedule next frame
    animationId = requestAnimationFrame(animate);
}

/**
 * Start the WebGPU engine
 */
export async function startEngine(): Promise<void> {
    // Initialize wasm-bindgen runtime
    await init();

    // Initialize WebGPU from Rust
    try {
        await init_gpu('gpu-canvas');
        console.log('WebGPU initialized with skeleton pipeline!');

        // Test the add function
        console.log(`Wasm test: 2 + 3 = ${add(2, 3)}`);

        // Initialize camera controls
        const canvas = document.getElementById('gpu-canvas') as HTMLCanvasElement;
        if (canvas) {
            initCameraControls(canvas);
            console.log('Camera controls initialized');
        }

        // Start the animation loop
        animationId = requestAnimationFrame(animate);
        log('Animation loop started');

        // Handle window resize
        window.addEventListener('resize', () => {
            console.log('Window resize event fired');
            resize_gpu('gpu-canvas');
        });
    } catch (e) {
        console.error('WebGPU initialization failed:', e);
    }
}

/**
 * Stop the animation loop
 */
export function stopEngine(): void {
    if (animationId !== null) {
        cancelAnimationFrame(animationId);
        animationId = null;
        initialized = false;
        log('Animation loop stopped');
    }
}
