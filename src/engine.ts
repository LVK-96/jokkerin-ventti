import init, { init_gpu, render_frame, resize_surface, advance_time, log } from '../wasm/pkg/jokkerin_ventti_wasm';
import { initCameraControls, updateCameraFromInput } from './camera';

export type UpdateCallback = (delta: number) => void;

export class WebGPUEngine {
    private animationId: number | null = null;
    private lastTime = 0;
    private initialized = false;
    private onUpdate: UpdateCallback | null = null;
    private canvasId: string;

    // FPS counter state
    private fpsElement: HTMLElement | null = null;
    private frameCount = 0;
    private fpsLastUpdate = 0;
    private showFps = true;

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

            // Create FPS counter element
            this.createFpsCounter();

            this.initialized = true;
        } catch (e) {
            console.error('WebGPU initialization failed:', e);
            throw e;
        }
    }

    /**
     * Create FPS counter DOM element
     */
    private createFpsCounter(): void {
        this.fpsElement = document.createElement('div');
        this.fpsElement.id = 'fps-counter';
        this.fpsElement.style.cssText = `
            position: fixed;
            bottom: 8px;
            right: 8px;
            background: rgba(0, 0, 0, 0.6);
            color: #ffcc00;
            font-family: monospace;
            font-size: 14px;
            padding: 4px 8px;
            border-radius: 4px;
            border: none;
            outline: none;
            box-shadow: none;
            text-shadow: none;
            -webkit-text-stroke: 0;
            z-index: 9999;
            pointer-events: none;
        `;
        this.fpsElement.textContent = '-- FPS';
        document.body.appendChild(this.fpsElement);

        if (!this.showFps) {
            this.fpsElement.style.display = 'none';
        }
    }

    /**
     * Toggle FPS counter visibility
     */
    setFpsVisible(visible: boolean): void {
        this.showFps = visible;
        if (this.fpsElement) {
            this.fpsElement.style.display = visible ? 'block' : 'none';
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
        this.fpsLastUpdate = this.lastTime;
        this.frameCount = 0;
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

        // Update FPS counter
        this.frameCount++;
        const fpsDelta = time - this.fpsLastUpdate;
        if (fpsDelta >= 1000) {
            const fps = Math.round((this.frameCount * 1000) / fpsDelta);
            const avgFrameTime = (fpsDelta / this.frameCount).toFixed(1);
            if (this.fpsElement) {
                this.fpsElement.textContent = `${fps} FPS | ${avgFrameTime}ms`;
            }
            this.frameCount = 0;
            this.fpsLastUpdate = time;
        }

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
