import { Exercise, Workout } from './types';
import { startEngine, set_exercise, load_animation } from './webgpu';

// Assets
import workoutUrl from './assets/Workouts/jokkeri_ventti.json?url';
import longBeepUrl from './assets/long_beep.mp3';
import almostSoundUrl from './assets/almostSound.mp3';
import shortBeepUrl from './assets/short_beep.mp3';
import intermediateSoundUrl from './assets/intermediateSound.mp3';

// Animation data - loaded as raw text for wasm parsing
import jumpingJacksAnim from './assets/animations/jumping_jacks.json?raw';
import lungesAnim from './assets/animations/lunges.json?raw';

// Workout configuration
const WORKOUT_JSON_PATH = workoutUrl;

// State
let exercises: Exercise[] = [];
let currentExercise = 0;
let workoutDone = false;
let intervalId: number | null = null;
let wakeLock: WakeLockSentinel | null = null;

let exerciseName = '';
let pauseTimer = 10;
let currentSet = 0;
let workoutTimer = 0;
let pauseState = true;

// Audio elements
const startSound = new Audio(longBeepUrl);
const almostPauseSound = new Audio(almostSoundUrl);
const almostStartSound = new Audio(almostSoundUrl);
const pauseSound = new Audio(shortBeepUrl);
const intermediateSound = new Audio(intermediateSoundUrl);

// Intermediate sounds
let intermBeeps: number[] = [];
let intermBeepsIdx = -1;

// Colors
const COLOR_REST = '#ffe7cd';
const COLOR_WORKOUT = '#d7ffce';

// State-machine states
const STATE_NEW_SET = 0;
const STATE_WORKOUT = 1;
const STATE_REST = 2;
let state = STATE_NEW_SET;

// UI element references
let elementsWithText: Array<{ element: HTMLElement; originalSize: number }> = [];

async function loadWorkout(): Promise<void> {
    const response = await fetch(WORKOUT_JSON_PATH);
    const workout: Workout = await response.json();
    exercises = workout.exercises;
    exerciseName = exercises[0].name;
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

function updateUI(): void {
    if (workoutDone || currentExercise >= exercises.length) {
        return;
    }

    const progressBar = document.getElementById('progress-bar') as HTMLElement;
    let timerText = '';

    if (!pauseState) {
        timerText = `${workoutTimer}`;
        const totalTime = exercises[currentExercise].workoutTime;
        const progressPercentage = ((totalTime - workoutTimer) / totalTime) * 100;
        progressBar.style.width = `${progressPercentage}%`;
        progressBar.style.backgroundColor = '#4caf50';
    } else {
        timerText = `${pauseTimer}`;
        const totalTime = exercises[currentExercise].pauseTime;
        const progressPercentage = (pauseTimer / totalTime) * 100;
        progressBar.style.width = `${progressPercentage}%`;
        progressBar.style.backgroundColor = '#ff9800';
    }

    const exerciseCountEl = document.getElementById('exercise-count') as HTMLElement;
    const exerciseNameEl = document.getElementById('exercise-name') as HTMLElement;
    const setCountEl = document.getElementById('set-count') as HTMLElement;
    const timerEl = document.getElementById('timer') as HTMLElement;

    // Update text: Initial start phase
    if (currentSet === 0) {
        exerciseCountEl.innerText = 'Next';
        exerciseNameEl.innerText = exercises[currentExercise].name;
    }
    // Update text: Last set pause
    else if (pauseState && currentSet === exercises[currentExercise].setCount && currentExercise < exercises.length - 1) {
        exerciseCountEl.innerText = 'Next';
        exerciseNameEl.innerText = exercises[currentExercise + 1].name;
    }
    // Update text: Normal
    else {
        exerciseCountEl.innerText = `Exercise ${currentExercise + 1} of ${exercises.length}`;
        exerciseNameEl.innerText = exerciseName;
    }

    setCountEl.innerText = `Set ${currentSet} of ${exercises[currentExercise].setCount}`;
    timerEl.innerText = timerText;
}

function finished(): void {
    releaseWakeLock();
    if (intervalId !== null) {
        clearInterval(intervalId);
    }

    const exerciseCountEl = document.getElementById('exercise-count') as HTMLElement;
    const exerciseNameEl = document.getElementById('exercise-name') as HTMLElement;
    const timerEl = document.getElementById('timer') as HTMLElement;
    const setCountEl = document.getElementById('set-count') as HTMLElement;

    exerciseCountEl.innerText = `Exercise ${currentExercise} of ${exercises.length}`;
    exerciseNameEl.innerText = 'Workout Done';
    timerEl.innerText = '';
    setCountEl.innerText = '';
}

function nextExercise(): void {
    currentSet = 1;
    currentExercise++;

    if (currentExercise >= exercises.length) {
        currentExercise = exercises.length;
        finished();
        return;
    }

    workoutTimer = exercises[currentExercise].workoutTime;
    pauseTimer = exercises[currentExercise].pauseTime;
    pauseState = false;

    // Update stickman animation for current exercise
    set_exercise(exercises[currentExercise].name);

    updateUI();
}

function updateSound(): void {
    if (state === STATE_NEW_SET) {
        if (!pauseState && workoutTimer === exercises[currentExercise].workoutTime) {
            startSound.play();
        }

        intermBeepsIdx = -1;
        const beeps = exercises[currentExercise].intermediateBeeps;

        if (beeps !== undefined) {
            intermBeeps = [...beeps].sort((a, b) => a - b);
            intermBeepsIdx = intermBeeps.length - 1;
        }
    } else if (state === STATE_WORKOUT) {
        if (workoutTimer === 0) {
            pauseSound.play();
        } else if (workoutTimer <= 3 && workoutTimer > 0) {
            almostPauseSound.play();
        }

        if (intermBeepsIdx >= 0 && workoutTimer <= intermBeeps[intermBeepsIdx]) {
            intermediateSound.play();
            intermBeepsIdx--;
        }
    } else if (state === STATE_REST) {
        if (pauseTimer <= 3 && pauseTimer > 0) {
            almostStartSound.play();
        }
    }
}

function updateColors(): void {
    if (state === STATE_NEW_SET) {
        document.body.style.backgroundColor = COLOR_WORKOUT;
    } else if (state === STATE_REST) {
        document.body.style.backgroundColor = COLOR_REST;
    } else if (state === STATE_WORKOUT) {
        document.body.style.backgroundColor = workoutTimer > 0 ? COLOR_WORKOUT : COLOR_REST;
    }
}

function statemachine(): void {
    if (workoutTimer > 0) {
        pauseState = false;
        workoutTimer--;
        state = STATE_WORKOUT;
    } else if (pauseTimer - 1 > 0 && currentExercise < exercises.length - 1) {
        pauseState = true;
        pauseTimer--;
        state = STATE_REST;
    } else {
        currentSet++;
        if (currentSet > exercises[currentExercise].setCount) {
            nextExercise();
        } else {
            workoutTimer = exercises[currentExercise].workoutTime;
            pauseTimer = exercises[currentExercise].pauseTime;
        }

        if (currentExercise < exercises.length) {
            exerciseName = exercises[currentExercise].name;
        }
        pauseState = false;
        state = STATE_NEW_SET;
    }

    updateSound();
    updateUI();
    updateColors();
}

function storeElementsWithText(): void {
    elementsWithText = Array.from(document.querySelectorAll('*'))
        .filter((element): element is HTMLElement => {
            return element instanceof HTMLElement && element.textContent?.trim().length !== 0;
        })
        .map((element) => {
            const computedStyle = window.getComputedStyle(element);
            const originalSize = parseFloat(computedStyle.getPropertyValue('font-size'));
            return { element, originalSize };
        });
}

function resetElementsWithText(): void {
    elementsWithText = [];
}

function updateTextSizes(percentage: number): void {
    elementsWithText.forEach(({ element, originalSize }) => {
        const newSize = originalSize * (1 + percentage / 100);
        element.style.fontSize = newSize + 'px';
    });
}

function updateTextSize(): void {
    const slider = document.getElementById('textSizeSlider') as HTMLInputElement;
    const sliderValue = parseInt(slider.value);
    updateTextSizes(sliderValue);
}

/**
 * Skip to a specific exercise by index (0-based)
 */
function skipToExercise(index: number): void {
    if (index < 0 || index >= exercises.length) {
        console.log(`Invalid exercise index: ${index}`);
        return;
    }

    currentExercise = index;
    currentSet = 1;
    workoutTimer = exercises[currentExercise].workoutTime;
    pauseTimer = exercises[currentExercise].pauseTime;
    pauseState = false;
    exerciseName = exercises[currentExercise].name;

    // Update stickman animation for current exercise
    set_exercise(exercises[currentExercise].name);
    console.log(`Skipped to exercise ${index + 1}: ${exerciseName}`);

    updateUI();
}

/**
 * Skip to next exercise
 */
function skipNextExercise(): void {
    if (currentExercise < exercises.length - 1) {
        skipToExercise(currentExercise + 1);
    }
}

/**
 * Skip to previous exercise
 */
function skipPrevExercise(): void {
    if (currentExercise > 0) {
        skipToExercise(currentExercise - 1);
    }
}

// Keyboard shortcuts for exercise navigation
document.addEventListener('keydown', (event) => {
    // Skip to next exercise with 'n' or right arrow
    if (event.key === 'n' || event.key === 'N' || event.key === 'ArrowRight') {
        skipNextExercise();
    }
    // Skip to previous exercise with 'p' or left arrow
    if (event.key === 'p' || event.key === 'P' || event.key === 'ArrowLeft') {
        skipPrevExercise();
    }
});

function startWorkout(): void {
    requestWakeLock();
    updateUI();

    updateTextSizes(0);
    resetElementsWithText();
    storeElementsWithText();
    updateTextSize();

    // Start the first exercise animation
    set_exercise(exercises[currentExercise].name);

    workoutDone = false;
    intervalId = window.setInterval(() => {
        requestWakeLock();
        statemachine();
    }, 1000);
}

// Initialize
async function init(): Promise<void> {
    await loadWorkout();

    // Initialize WebGPU + Wasm engine
    await startEngine();

    // Load keyframe animations for exercises
    load_animation(jumpingJacksAnim);
    load_animation(lungesAnim);

    resetElementsWithText();
    storeElementsWithText();

    const startButton = document.getElementById('startButton') as HTMLButtonElement;
    startButton.addEventListener('click', () => {
        startButton.hidden = true;
        pauseSound.play();

        (document.getElementById('progress-bar-container') as HTMLElement).hidden = false;
        (document.getElementById('progress-bar') as HTMLElement).hidden = false;
        (document.getElementById('timer') as HTMLElement).hidden = false;

        startWorkout();
    }, { once: true });

    let startPressed = false;
    document.addEventListener('mouseup', () => {
        if (startPressed || !startButton.hidden) return;
        startPressed = true;

        const slider = document.getElementById('textSizeSlider') as HTMLElement;
        slider.hidden = !slider.hidden;
    });
}

// Expose updateTextSize globally for the inline handler
(window as unknown as { updateTextSize: typeof updateTextSize }).updateTextSize = updateTextSize;

init();
