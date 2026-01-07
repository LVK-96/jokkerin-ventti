import init, { init_gpu, App } from '../wasm/pkg/jokkerin_ventti_wasm';
import { initCameraControls, updateCameraFromInput } from './camera';

export type UpdateCallback = (delta: number, app: App) => void;
export type StatsCallback = (fps: number, avgFrameTime: string) => void;
export type ErrorCallback = (error: unknown, title: string) => void;

export class WebGPUEngine {
    private app: App | null = null;
    private animationId: number | null = null;
    private lastTime = 0;
    private initialized = false;
    private onUpdate: UpdateCallback | null = null;
    private onStats: StatsCallback | null = null;
    private onError: ErrorCallback | null = null;
    private canvasId: string;

    // FPS counter state
    private frameCount = 0;
    private fpsLastUpdate = 0;

    constructor(canvasId: string) {
        this.canvasId = canvasId;
    }

    public setStatsCallback(cb: StatsCallback) {
        this.onStats = cb;
    }

    public setErrorCallback(cb: ErrorCallback) {
        this.onError = cb;
    }

    /**
     * Initialize the WebGPU engine and Wasm module
     */
    async init(): Promise<void> {
        if (this.initialized) return;

        try {
            // Initialize wasm-bindgen runtime
            await init();

            // Detect if WebGPU is available to decide initial attempt
            const hasWebGPU = 'gpu' in navigator;
            let app: App;

            try {
                // Try WebGPU first unless it's definitely missing
                const forceWebGL = !hasWebGPU;
                if (!hasWebGPU) {
                    console.log('WebGPU not detected, defaulting to WebGL');
                }
                app = await init_gpu(this.canvasId, forceWebGL);
                console.log(`Graphics initialized via ${forceWebGL ? 'WebGL' : 'WebGPU'}`);
            } catch (e) {
                // If WebGPU failed (even if checking navigator.gpu passed), try forcing WebGL
                console.warn('WebGPU initialization failed, attempting fallback to WebGL:', e);

                // CRITICAL: The failed WebGPU attempt might have locked the canvas context.
                // We must recreate the canvas element to get a fresh context for WebGL.
                const oldCanvas = document.getElementById(this.canvasId);
                if (oldCanvas && oldCanvas.parentNode) {
                    const newCanvas = oldCanvas.cloneNode(true) as HTMLCanvasElement;
                    oldCanvas.parentNode.replaceChild(newCanvas, oldCanvas);
                    console.log('Recreated canvas element to clear context locks.');
                }

                try {
                    app = await init_gpu(this.canvasId, true);
                    console.log('Fallback to WebGL successful');
                } catch (e2) {
                    console.error('WebGL fallback also failed:', e2);

                    if (this.onError) {
                        this.onError(e2, "Graphics Initialization Failed");
                    }
                    throw e2;
                }
            }
            this.app = app;

            // Initialize camera controls
            const canvas = document.getElementById(this.canvasId) as HTMLCanvasElement;
            if (canvas && this.app) {
                initCameraControls(canvas, this.app);
                console.log('Camera controls initialized');
            }

            // Handle window resize
            window.addEventListener('resize', () => {
                try {
                    this.app?.resize_surface(this.canvasId);
                } catch (e) {
                    console.error('Resize failed:', e);
                }
            });

            this.initialized = true;
        } catch (e) {
            console.error('WebGPU initialization failed:', e);
            throw e;
        }
    }

    /**
     * Get the WASM App instance for direct access to WASM functions
     */
    get wasmApp(): App | null {
        return this.app;
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
        this.fpsLastUpdate = this.lastTime;
        this.frameCount = 0;
        this.loop(this.lastTime);
        console.log('Animation loop started');
    }

    /**
     * Stop the animation loop
     */
    stop(): void {
        if (this.animationId !== null) {
            cancelAnimationFrame(this.animationId);
            this.animationId = null;
            console.log('Animation loop stopped');
        }
    }

    private loop = (time: number): void => {
        const delta = time - this.lastTime;
        this.lastTime = time;

        // Update FPS counter
        this.frameCount++;
        const fpsDelta = time - this.fpsLastUpdate;
        if (fpsDelta >= 1000) {
            const fps = Math.round((this.frameCount * 1000) / fpsDelta);
            const avgFrameTime = (fpsDelta / this.frameCount).toFixed(1);

            if (this.onStats) {
                this.onStats(fps, avgFrameTime);
            }

            this.frameCount = 0;
            this.fpsLastUpdate = time;
        }

        try {
            if (!this.app) return;

            // Update time uniform in Rust
            this.app.advance_time(delta);

            // Update camera from input
            updateCameraFromInput();

            // Run custom update logic (e.g., skeleton playback or editor session)
            if (this.onUpdate) {
                this.onUpdate(delta, this.app);
            }

            // Render the frame
            this.app.render_frame();

            // Schedule next frame
            this.animationId = requestAnimationFrame(this.loop);
        } catch (e) {
            console.error('Error in animation loop:', e);
            this.stop();
        }
    };
}
