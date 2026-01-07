import { Exercise, Workout } from './types';
import { AudioManager } from './audio-manager';
import { UIController } from './ui-controller';
import { WebGPUEngine } from './engine';
import { WorkoutState, WorkoutPhase, createInitialState, tick, WorkoutEvent } from './workout-state';
import { resolveAnimationId, animationData, fetchAnimationBuffer } from './animations';
import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';
import workoutUrl from './assets/Workouts/jokkeri_ventti.json?url';

const WORKOUT_JSON_PATH = workoutUrl;

export class WorkoutApp {
    private exercises: Exercise[] = [];
    private state: WorkoutState;
    private intervalId: number | null = null;
    private wakeLock: WakeLockSentinel | null = null;

    private audio: AudioManager;
    private ui: UIController;
    private engine: WebGPUEngine;

    constructor() {
        this.audio = new AudioManager();
        this.ui = new UIController();
        this.engine = new WebGPUEngine('gpu-canvas');

        // Initial state placeholder
        this.state = {
            phase: WorkoutPhase.Finished,
            exerciseIndex: 0,
            setIndex: 0,
            workoutTimer: 0,
            pauseTimer: 0
        };
    }

    public async init(): Promise<void> {
        try {
            await this.loadWorkout();

            // Wire up Engine callbacks
            this.engine.setStatsCallback((fps, ms) => this.ui.updateFps(fps, ms));
            this.engine.setErrorCallback((err, msg) => this.ui.showFatalError(msg, String(err)));

            // Init Engine
            await this.engine.init();
            this.engine.start((_delta, app) => {
                app.update_skeleton_from_playback();
            });

            // Load animations
            await this.loadAnimations();

            // Preload Audio
            await this.audio.preload();

            // Bind UI
            this.ui.attachStartHandler(() => this.startWorkout());
            this.ui.attachNavigationHandlers(
                () => this.skipPrevExercise(),
                () => this.skipNextExercise()
            );

            // Bind inputs
            this.bindKeyboardShortcuts();
            this.bindSliderToggle();

            // Initial UI update
            this.ui.reset();

            console.log("WorkoutApp initialized");
        } catch (e) {
            console.error("Failed to initialize WorkoutApp:", e);
            this.ui.showFatalError("Application Error", "Failed to initialize application", String(e));
        }
    }

    private async loadWorkout(): Promise<void> {
        const response = await fetch(WORKOUT_JSON_PATH);
        const workout: Workout = await response.json();
        this.exercises = workout.exercises;

        // Resolve animation IDs
        this.exercises.forEach(ex => {
            const resolvedId = resolveAnimationId(ex.name);
            if (resolvedId !== undefined) {
                ex.animationId = resolvedId;
            } else {
                console.warn(`Could not resolve animation ID for exercise: ${ex.name}`);
            }
        });

        // Starting state with loaded exercises
        this.state = createInitialState(this.exercises);
    }

    private async loadAnimations(): Promise<void> {
        const app = this.engine.wasmApp;
        if (!app) return;

        const loadPromises = Object.entries(animationData).map(async ([idStr, url]) => {
            const id = Number(idStr) as AnimationId;
            try {
                const buffer = await fetchAnimationBuffer(url);
                app.load_animation_binary(id, buffer);
            } catch (err) {
                console.error(`Failed to load animation ${id}:`, err);
            }
        });
        await Promise.all(loadPromises);
    }

    private startWorkout() {
        this.requestWakeLock();
        this.state = createInitialState(this.exercises);

        // Upload first animation immediately
        this.syncAnimation();

        this.ui.showWorkoutScreen();
        this.ui.update(this.state, this.exercises);

        if (this.intervalId !== null) clearInterval(this.intervalId);
        this.intervalId = window.setInterval(() => this.tickLoop(), 1000);
    }

    public stopAndResetWorkout(): void {
        if (this.intervalId !== null) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }

        this.releaseWakeLock();
        this.state = createInitialState(this.exercises);

        this.ui.reset();
        this.ui.attachStartHandler(() => this.startWorkout());

        console.log('Workout stopped and reset');
    }

    private tickLoop() {
        const result = tick(this.state, this.exercises);
        this.state = result.newState;
        this.handleEvents(result.events);
        this.ui.update(this.state, this.exercises);
    }

    private handleEvents(events: WorkoutEvent[]) {
        for (const event of events) {
            switch (event.type) {
                case 'play_sound':
                    this.audio.play(event.sound);
                    break;
                case 'start_exercise':
                    this.syncAnimation();
                    break;
                case 'finished':
                    this.stopAndResetWorkout();
                    break;
            }
        }
    }

    private syncAnimation() {
        if (this.exercises.length === 0) return;

        const exercise = this.exercises[this.state.exerciseIndex];
        const app = this.engine.wasmApp;
        if (exercise.animationId !== undefined && app) {
            app.set_exercise(exercise.animationId);
        }
    }

    private skipToExercise(index: number): void {
        if (index < 0 || index >= this.exercises.length) {
            console.log(`Invalid exercise index: ${index}`);
            return;
        }

        // Manual state transition
        this.state = {
            phase: WorkoutPhase.Workout,
            exerciseIndex: index,
            setIndex: 1,
            workoutTimer: this.exercises[index].workoutTime,
            pauseTimer: this.exercises[index].pauseTime
        };

        const exercise = this.exercises[index];
        console.log(`Skipped to exercise ${index + 1}: ${exercise.name}`);

        this.syncAnimation();
        this.ui.update(this.state, this.exercises);
    }

    private skipNextExercise(): void {
        if (this.state.exerciseIndex < this.exercises.length - 1) {
            this.skipToExercise(this.state.exerciseIndex + 1);
        }
    }

    private skipPrevExercise(): void {
        if (this.state.exerciseIndex > 0) {
            this.skipToExercise(this.state.exerciseIndex - 1);
        }
    }

    // --- Inputs ---

    private bindKeyboardShortcuts() {
        document.addEventListener('keydown', (event) => {
            // Only if workout is started (i.e. start button hidden)
            if (!this.ui.isStartButtonHidden()) return;

            if (event.key === 'n' || event.key === 'N') {
                this.skipNextExercise();
            }
            if (event.key === 'p' || event.key === 'P') {
                this.skipPrevExercise();
            }
        });
    }

    private bindSliderToggle() {
        document.addEventListener('mouseup', () => {
            // Sync local var with UI state if needed, but UIController tells us truth
            if (!this.ui.isStartButtonHidden()) return;

            // Toggle
            this.ui.toggleTextSizeSlider();
        });
    }

    // --- Wake Lock ---

    private async requestWakeLock(): Promise<void> {
        try {
            this.wakeLock = await navigator.wakeLock.request('screen');
        } catch (err) {
            console.error(`Error while acquiring wake lock: ${err}`);
        }
    }

    private async releaseWakeLock(): Promise<void> {
        if (this.wakeLock !== null) {
            await this.wakeLock.release();
            this.wakeLock = null;
            console.log('Screen wake lock released');
        }
    }
}
