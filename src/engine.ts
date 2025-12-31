import init, { init_gpu, render_frame, resize_surface, advance_time, log } from '../wasm/pkg/jokkerin_ventti_wasm';
import { initCameraControls, updateCameraFromInput } from './camera';

export type UpdateCallback = (delta: number) => void;

export class WebGPUEngine {
    private animationId: number | null = null;
    private lastTime = 0;
    private initialized = false;
    private onUpdate: UpdateCallback | null = null;
    private canvasId: string;

    constructor(canvasId: string) {
        this.canvasId = canvasId;
    }

    /**
     * Initialize the WebGPU engine and Wasm module
     */
    async init(): Promise<void> {
        if (this.initialized) return;

        try {
            // Initialize wasm-bindgen runtime
            await init();

            // Initialize WebGPU from Rust
            await init_gpu(this.canvasId);
            console.log(`WebGPU initialized on canvas '${this.canvasId}'`);

            // Initialize camera controls
            const canvas = document.getElementById(this.canvasId) as HTMLCanvasElement;
            if (canvas) {
                initCameraControls(canvas);
                console.log('Camera controls initialized');
            }

            // Handle window resize
            window.addEventListener('resize', () => {
                resize_surface(this.canvasId);
            });

            this.initialized = true;
        } catch (e) {
            console.error('WebGPU initialization failed:', e);
            throw e;
        }
    }

    /**
     * Start the animation loop
     * @param onUpdate Callback function to run before rendering each frame
     */
    start(onUpdate?: UpdateCallback): void {
        if (!this.initialized) {
            console.warn('Engine not initialized. Call init() first.');
            return;
        }

        if (onUpdate) {
            this.onUpdate = onUpdate;
        }

        if (this.animationId !== null) {
            return; // Already running
        }

        this.lastTime = performance.now();
        this.loop(this.lastTime);
        log('Animation loop started');
    }

    /**
     * Stop the animation loop
     */
    stop(): void {
        if (this.animationId !== null) {
            cancelAnimationFrame(this.animationId);
            this.animationId = null;
            log('Animation loop stopped');
        }
    }

    private loop = (time: number): void => {
        const delta = time - this.lastTime;
        this.lastTime = time;

        try {
            // Update time uniform in Rust
            advance_time(delta);

            // Update camera from keyboard input
            updateCameraFromInput();

            // Run custom update logic (e.g., skeleton playback or editor session)
            if (this.onUpdate) {
                this.onUpdate(delta);
            }

            // Render the frame
            render_frame();

            // Schedule next frame
            this.animationId = requestAnimationFrame(this.loop);
        } catch (e) {
            console.error('Error in animation loop:', e);
            this.stop();
        }
    };
}
