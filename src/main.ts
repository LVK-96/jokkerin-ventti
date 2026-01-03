import { Exercise, Workout } from './types';
import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';
import { WebGPUEngine } from './engine';
import { resolveAnimationId, animationData } from './animations';
import { AudioManager } from './audio-manager';
import { UIController } from './ui-controller';
import { WorkoutState, WorkoutPhase, createInitialState, tick, WorkoutEvent } from './workout-state';

// Assets
import workoutUrl from './assets/Workouts/jokkeri_ventti.json?url';

// Configuration
const WORKOUT_JSON_PATH = workoutUrl;

// State
let exercises: Exercise[] = [];
export let exerciseOrder: string[] = []; // Exercise names in workout order (for editor)
let state: WorkoutState = {
    phase: WorkoutPhase.Finished,
    exerciseIndex: 0,
    setIndex: 0,
    workoutTimer: 0,
    pauseTimer: 0
};

let intervalId: number | null = null;
let wakeLock: WakeLockSentinel | null = null;

// Subsystems
const audio = new AudioManager();
const ui = new UIController();
let engine: WebGPUEngine | null = null;

async function loadWorkout(): Promise<void> {
    const response = await fetch(WORKOUT_JSON_PATH);
    const workout: Workout = await response.json();
    exercises = workout.exercises;

    // Resolve animation IDs at load time
    exercises.forEach(ex => {
        const resolvedId = resolveAnimationId(ex.name);
        if (resolvedId !== undefined) {
            ex.animationId = resolvedId;
        } else {
            console.warn(`Could not resolve animation ID for exercise: ${ex.name}`);
        }
    });

    // Populate exercise order for the keyframe editor
    exerciseOrder = exercises.map(e => e.name);
    state = createInitialState(exercises);
}

async function requestWakeLock(): Promise<void> {
    try {
        wakeLock = await navigator.wakeLock.request('screen');
    } catch (err) {
        console.error(`Error while acquiring wake lock: ${err}`);
    }
}

async function releaseWakeLock(): Promise<void> {
    if (wakeLock !== null) {
        await wakeLock.release();
        wakeLock = null;
        console.log('Screen wake lock released');
    }
}

function handleEvents(events: WorkoutEvent[]) {
    for (const event of events) {
        switch (event.type) {
            case 'play_sound':
                audio.play(event.sound);
                break;
            case 'start_exercise':
                {
                    const animId = exercises[state.exerciseIndex].animationId;
                    if (animId !== undefined && engine?.wasmApp) {
                        engine.wasmApp.set_exercise(animId);
                    }
                }
                break;
            case 'finished':
                stopAndResetWorkout();

                if (intervalId !== null) {
                    clearInterval(intervalId);
                    intervalId = null;
                }

                releaseWakeLock();
                break;
        }
    }
}

function tickLoop() {
    const result = tick(state, exercises);
    state = result.newState;
    handleEvents(result.events);
    ui.update(state, exercises);
}

function startWorkout() {
    requestWakeLock();
    // Reset state to initial ready state
    state = createInitialState(exercises);

    // Initialize animation immediately
    if (exercises.length > 0) {
        const exercise = exercises[state.exerciseIndex];
        if (exercise.animationId !== undefined && engine?.wasmApp) {
            engine.wasmApp.set_exercise(exercise.animationId);
        }
    }

    ui.showWorkoutScreen();
    ui.update(state, exercises);

    // Start loop
    if (intervalId !== null) clearInterval(intervalId);
    intervalId = window.setInterval(tickLoop, 1000);
}

/**
 * Stop and reset the workout to initial state.
 * Called when entering editor mode to ensure clean slate.
 */
export function stopAndResetWorkout(): void {
    if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
    }

    releaseWakeLock();

    // Reset state
    state = createInitialState(exercises);

    // Reset UI
    ui.reset();
    ui.attachStartHandler(startWorkout);

    console.log('Workout stopped and reset');
}

/**
 * Skip to a specific exercise by index (0-based)
 */
function skipToExercise(index: number): void {
    if (index < 0 || index >= exercises.length) {
        console.log(`Invalid exercise index: ${index}`);
        return;
    }

    // Manual state transition
    state = {
        phase: WorkoutPhase.Workout,
        exerciseIndex: index,
        setIndex: 1,
        workoutTimer: exercises[index].workoutTime,
        pauseTimer: exercises[index].pauseTime
    };

    const exercise = exercises[index];
    if (exercise.animationId !== undefined && engine?.wasmApp) {
        engine.wasmApp.set_exercise(exercise.animationId);
    }
    console.log(`Skipped to exercise ${index + 1}: ${name}`);

    ui.update(state, exercises);
}

function skipNextExercise(): void {
    if (state.exerciseIndex < exercises.length - 1) {
        skipToExercise(state.exerciseIndex + 1);
    }
}

function skipPrevExercise(): void {
    if (state.exerciseIndex > 0) {
        skipToExercise(state.exerciseIndex - 1);
    }
}

// Global hook for the slider
(window as unknown as { updateTextSize: () => void }).updateTextSize = () => {
    ui.updateTextSize();
};

async function init(): Promise<void> {
    await loadWorkout();

    // Initialize WebGPU + Wasm engine
    engine = new WebGPUEngine('gpu-canvas');
    await engine.init();

    engine.start((_delta, app) => {
        app.update_skeleton_from_playback();
    });

    // Load keyframe animations for exercises
    const app = engine.wasmApp;
    if (app) {
        for (const [idStr, animJson] of Object.entries(animationData)) {
            const id = Number(idStr) as AnimationId;
            app.load_animation(id, animJson);
        }
    }

    // Initialize UI handlers
    ui.attachStartHandler(startWorkout);
    ui.attachNavigationHandlers(
        () => skipPrevExercise(),
        () => skipNextExercise()
    );

    // Keyboard shortcuts
    document.addEventListener('keydown', (event) => {
        // Only if workout is started (i.e. start button hidden)
        if (!ui.isStartButtonHidden()) return;

        if (event.key === 'n' || event.key === 'N') {
            skipNextExercise();
        }
        if (event.key === 'p' || event.key === 'P') {
            skipPrevExercise();
        }
    });

    // Toggle slider visibility on click (logic from original main.ts)
    // "document.addEventListener('mouseup', ...)"
    // Using a simpler approach if possible, but sticking to original logic
    let startPressed = false;
    document.addEventListener('mouseup', () => {
        if (startPressed || !ui.isStartButtonHidden()) return;
        startPressed = true;
        ui.toggleTextSizeSlider();
    });
}

init();
