/**
 * WebGPU + Wasm integration module
 *
 * Loads the Rust Wasm module and initializes WebGPU from Rust.
 */

import init, { init_gpu, render_frame, add, log } from '../wasm/pkg/jokkerin_ventti_wasm';

/**
 * Start the WebGPU engine
 */
export async function startEngine(): Promise<void> {
    // Initialize wasm-bindgen runtime
    await init();

    // Initialize WebGPU from Rust
    try {
        init_gpu('gl-canvas');

        // Give the async GPU initialization time to complete
        await new Promise(resolve => setTimeout(resolve, 100));

        render_frame();
        log('WebGPU engine started from TypeScript');

        // Test the add function
        console.log(`Wasm test: 2 + 3 = ${add(2, 3)}`);
    } catch (e) {
        console.error('WebGPU initialization failed:', e);
    }
}
