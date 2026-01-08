import { Exercise } from './types';
import { WorkoutState, WorkoutPhase } from './workout-state';

export class UIController {
    // Elements
    private startButton: HTMLButtonElement | null;
    private progressBarContainer: HTMLElement | null;
    private progressBar: HTMLElement | null;
    private timerEl: HTMLElement | null;
    private prevButton: HTMLButtonElement | null;
    private nextButton: HTMLButtonElement | null;
    private exerciseCountEl: HTMLElement | null;
    private exerciseNameEl: HTMLElement | null;
    private setCountEl: HTMLElement | null;
    private textSizeSlider: HTMLInputElement | null;

    // State for text resizing
    private elementsWithText: Array<{ element: HTMLElement; originalSize: number }> = [];

    constructor() {
        this.startButton = document.getElementById('startButton') as HTMLButtonElement;
        this.progressBarContainer = document.getElementById('progress-bar-container');
        this.progressBar = document.getElementById('progress-bar');
        this.timerEl = document.getElementById('timer');
        this.prevButton = document.getElementById('prevButton') as HTMLButtonElement;
        this.nextButton = document.getElementById('nextButton') as HTMLButtonElement;
        this.exerciseCountEl = document.getElementById('exercise-count');
        this.exerciseNameEl = document.getElementById('exercise-name');
        this.setCountEl = document.getElementById('set-count');
        this.textSizeSlider = document.getElementById('textSizeSlider') as HTMLInputElement;

        this.initTextResizing();
    }

    private initTextResizing() {
        if (this.textSizeSlider) {
            this.textSizeSlider.addEventListener('input', () => this.updateTextSize());
        }
    }

    public attachStartHandler(handler: () => void) {
        if (this.startButton) {
            // Remove any existing listeners by cloning? No, just add unique listener.
            // Original code used { once: true }.
            this.startButton.addEventListener('click', (e) => {
                e.stopPropagation();
                handler();
            }, { once: true });
        }
    }

    public attachNavigationHandlers(onPrev: () => void, onNext: () => void) {
        if (this.prevButton) {
            this.prevButton.addEventListener('click', (e) => {
                e.stopPropagation();
                onPrev();
            });
        }
        if (this.nextButton) {
            this.nextButton.addEventListener('click', (e) => {
                e.stopPropagation();
                onNext();
            });
        }
    }

    public showWorkoutScreen() {
        if (this.startButton) this.startButton.hidden = true;
        if (this.progressBarContainer) this.progressBarContainer.hidden = false;
        if (this.progressBar) this.progressBar.hidden = false;
        if (this.timerEl) this.timerEl.hidden = false;
        if (this.prevButton) this.prevButton.hidden = false;
        if (this.nextButton) this.nextButton.hidden = false;

        // Text sizing logic
        this.updateTextSizes(0);
        this.resetElementsWithText();
        this.storeElementsWithText();
        this.updateTextSize();
    }

    public reset() {
        if (this.startButton) {
            this.startButton.hidden = false;
        }
        if (this.progressBarContainer) this.progressBarContainer.hidden = true;
        if (this.progressBar) this.progressBar.hidden = true;
        if (this.timerEl) this.timerEl.hidden = true;
        if (this.prevButton) this.prevButton.hidden = true;
        if (this.nextButton) this.nextButton.hidden = true;

        if (this.textSizeSlider) this.textSizeSlider.hidden = true;

        if (this.exerciseCountEl) this.exerciseCountEl.innerText = '';
        if (this.setCountEl) this.setCountEl.innerText = '';
        if (this.exerciseNameEl) {
            // Original code set this to first exercise name on reset if available
            // but we'll let the main controller handle that via update() or manual set.
        }

        document.body.style.backgroundColor = '#ffffff';

        // Initialize text resizing elements (captures startButton etc)
        this.resetElementsWithText();
        this.storeElementsWithText();
        this.updateTextSize();
    }

    public update(state: WorkoutState, exercises: Exercise[]) {
        const currentExercise = exercises[state.exerciseIndex];

        if (state.phase === WorkoutPhase.Finished) {
            this.showFinished(exercises.length);
            return;
        }

        const nextExerciseName = (state.exerciseIndex < exercises.length - 1)
            ? exercises[state.exerciseIndex + 1].name
            : null;

        // Colors
        this.updateColors(state);

        // Progress Bar & Timer
        let timerText = '';
        if (state.phase === WorkoutPhase.Ready) {
            // Ready phase: 10 second countdown before first exercise starts
            timerText = `${state.pauseTimer}`;
            if (this.progressBar) {
                const percentage = (state.pauseTimer / 10) * 100;
                this.progressBar.style.width = `${percentage}%`;
                this.progressBar.style.backgroundColor = '#ff9800'; // Orange (countdown color)
            }
        } else if (state.phase === WorkoutPhase.Rest) {
            timerText = `${state.pauseTimer}`;
            if (this.progressBar) {
                const percentage = (state.pauseTimer / currentExercise.pauseTime) * 100;
                this.progressBar.style.width = `${percentage}%`;
                this.progressBar.style.backgroundColor = '#ff9800'; // Orange
            }
        } else {
            // Workout
            timerText = `${state.workoutTimer}`;
            if (this.progressBar) {
                const total = currentExercise.workoutTime;
                const percentage = ((total - state.workoutTimer) / total) * 100;
                this.progressBar.style.width = `${percentage}%`;
                this.progressBar.style.backgroundColor = '#4caf50'; // Green
            }
        }
        if (this.timerEl) this.timerEl.innerText = timerText;

        // Text
        if (this.exerciseCountEl && this.exerciseNameEl) {
            if (state.phase === WorkoutPhase.Ready) {
                this.exerciseCountEl.innerText = 'Next';
                this.exerciseNameEl.innerText = currentExercise.name;
            } else if (state.phase === WorkoutPhase.Rest && state.setIndex === currentExercise.setCount && nextExerciseName) {
                this.exerciseCountEl.innerText = 'Next';
                this.exerciseNameEl.innerText = nextExerciseName;
            } else {
                this.exerciseCountEl.innerText = `Exercise ${state.exerciseIndex + 1} of ${exercises.length}`;
                this.exerciseNameEl.innerText = currentExercise.name;
            }
        }

        if (this.setCountEl) {
            this.setCountEl.innerText = `Set ${state.setIndex} of ${currentExercise.setCount}`;
        }

        // Buttons
        if (this.prevButton) this.prevButton.disabled = state.exerciseIndex <= 0;
        if (this.nextButton) this.nextButton.disabled = state.exerciseIndex >= exercises.length - 1;
    }

    private updateColors(state: WorkoutState) {
        const COLOR_REST = '#ffe7cd';
        const COLOR_WORKOUT = '#d7ffce';

        let color = '#ffffff';
        if (state.phase === WorkoutPhase.Ready) {
            color = COLOR_WORKOUT;
        } else if (state.phase === WorkoutPhase.Rest) {
            color = COLOR_REST;
        } else if (state.phase === WorkoutPhase.Workout) {
            color = state.workoutTimer > 0 ? COLOR_WORKOUT : COLOR_REST;
        }
        document.body.style.backgroundColor = color;
    }

    private showFinished(totalExercises: number) {
        if (this.exerciseCountEl) this.exerciseCountEl.innerText = `Exercise ${totalExercises} of ${totalExercises}`;
        if (this.exerciseNameEl) this.exerciseNameEl.innerText = 'Workout Done';
        if (this.timerEl) this.timerEl.innerText = '';
        if (this.setCountEl) this.setCountEl.innerText = '';
        if (this.prevButton) { this.prevButton.disabled = true; this.prevButton.hidden = true; }
        if (this.nextButton) { this.nextButton.disabled = true; this.nextButton.hidden = true; }
        document.body.style.backgroundColor = '#ffffff';
    }

    // Text Resizing Helpers
    private storeElementsWithText() {
        this.elementsWithText = Array.from(document.querySelectorAll('#timer, #exercise-name, #set-count, #exercise-count, #startButton'))
            .filter((element): element is HTMLElement => {
                return element instanceof HTMLElement;
            })
            .map((element) => {
                const computedStyle = window.getComputedStyle(element);
                const originalSize = parseFloat(computedStyle.getPropertyValue('font-size'));
                return { element, originalSize };
            });
    }

    private resetElementsWithText() {
        this.elementsWithText = [];
    }

    private updateTextSizes(percentage: number) {
        this.elementsWithText.forEach(({ element, originalSize }) => {
            const newSize = originalSize * (1 + percentage / 100);
            element.style.fontSize = newSize + 'px';
        });
    }

    public updateTextSize() {
        if (this.textSizeSlider) {
            const val = parseInt(this.textSizeSlider.value);
            this.updateTextSizes(val);
        }
    }

    public toggleTextSizeSlider() {
        if (this.textSizeSlider) {
            this.textSizeSlider.hidden = !this.textSizeSlider.hidden;
        }
    }

    public isStartButtonHidden(): boolean {
        return this.startButton ? this.startButton.hidden : true;
    }

    // --- System UI (Settings & FPS) ---

    private settingsButton: HTMLElement | null = null;
    private fpsElement: HTMLElement | null = null;
    private fpsVisible = false;

    public createSettingsButton() {
        if (this.settingsButton) return;

        this.settingsButton = document.createElement('div');
        this.settingsButton.id = 'settings-button';
        this.settingsButton.style.cssText = `
            position: fixed;
            bottom: 8px;
            right: 8px;
            background: rgba(0, 0, 0, 0.6);
            color: white;
            font-family: monospace;
            font-size: 18px;
            padding: 4px 8px;
            border-radius: 4px;
            cursor: pointer;
            z-index: 9999;
            user-select: none;
        `;
        this.settingsButton.textContent = '⚙️';
        this.settingsButton.title = 'Toggle text size slider';

        this.settingsButton.addEventListener('click', (e) => {
            e.stopPropagation();  // Prevent document mouseup from triggering
            this.toggleTextSizeSlider();
        });

        document.body.appendChild(this.settingsButton);
    }

    public createFpsCounter() {
        if (this.fpsElement) return;

        this.fpsElement = document.createElement('div');
        this.fpsElement.id = 'fps-counter';
        this.fpsElement.style.cssText = `
            position: fixed;
            bottom: 8px;
            right: 48px;
            background: rgba(0, 0, 0, 0.6);
            color: #ffcc00;
            font-family: monospace;
            font-size: 18px;
            padding: 4px 8px;
            border-radius: 4px;
            cursor: pointer;
            z-index: 9999;
            user-select: none;
        `;
        this.fpsElement.textContent = '📊';
        this.fpsElement.title = 'Toggle FPS stats (or press F)';

        this.fpsElement.addEventListener('click', (e) => {
            e.stopPropagation();  // Prevent document mouseup from triggering
            this.toggleFps();
        });

        document.body.appendChild(this.fpsElement);
    }

    public setFpsVisible(visible: boolean) {
        if (!this.fpsElement) this.createFpsCounter();
        this.fpsVisible = visible;
        if (!visible && this.fpsElement) {
            this.fpsElement.textContent = '📊';
        }
    }

    public updateFps(fps: number, avgFrameTime: string) {
        if (this.fpsElement && this.fpsVisible) {
            this.fpsElement.textContent = `${fps} FPS | ${avgFrameTime}ms`;
        }
    }

    public toggleFps(): void {
        this.fpsVisible = !this.fpsVisible;
        if (!this.fpsVisible && this.fpsElement) {
            this.fpsElement.textContent = '📊';
        }
    }

    public showFatalError(title: string, message: string, detail?: string) {
        const overlay = document.createElement('div');
        overlay.style.cssText = `
            position: fixed;
            top: 0; left: 0; width: 100%; height: 100%;
            background: rgba(0,0,0,0.85);
            color: white;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            z-index: 10000;
            font-family: sans-serif;
            text-align: center;
            padding: 20px;
        `;
        overlay.innerHTML = `
            <h2 style="color: #ff5555; margin-bottom: 10px;">${title}</h2>
            <p style="font-size: 1.1em; max-width: 600px; line-height: 1.5;">${message}</p>
            ${detail ? `
            <div style="margin-top: 20px; padding: 10px; background: #333; border-radius: 4px; font-family: monospace; font-size: 0.8em; color: #aaa;">
                ${detail}
            </div>` : ''}
        `;
        document.body.appendChild(overlay);
    }
}
