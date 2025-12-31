import { Exercise } from './types';

export enum WorkoutPhase {
    Ready = 'ready',
    Workout = 'workout',
    Rest = 'rest',
    Finished = 'finished'
}

export interface WorkoutState {
    phase: WorkoutPhase;
    exerciseIndex: number;
    setIndex: number;
    workoutTimer: number;
    pauseTimer: number;
}

export type WorkoutEvent =
    | { type: 'start_exercise', exerciseName: string }
    | { type: 'play_sound', sound: 'start' | 'pause' | 'almost_start' | 'almost_pause' | 'intermediate' }
    | { type: 'phase_change', phase: WorkoutPhase }
    | { type: 'finished' };

export function createInitialState(exercises: Exercise[]): WorkoutState {
    if (exercises.length === 0) {
        return {
            phase: WorkoutPhase.Finished,
            exerciseIndex: 0,
            setIndex: 0,
            workoutTimer: 0,
            pauseTimer: 0
        };
    }

    return {
        phase: WorkoutPhase.Ready,
        exerciseIndex: 0,
        setIndex: 1,
        workoutTimer: exercises[0].workoutTime,
        pauseTimer: exercises[0].pauseTime
    };
}

/**
 * Pure function to calculate the next state based on current state and exercises.
 * Returns the new state and any events that occurred during the transition.
 */
export function tick(state: WorkoutState, exercises: Exercise[]): { newState: WorkoutState, events: WorkoutEvent[] } {
    if (state.phase === WorkoutPhase.Finished) {
        return { newState: state, events: [] };
    }

    const newState = { ...state };
    const events: WorkoutEvent[] = [];
    const currentExercise = exercises[state.exerciseIndex];

    // Transition from Ready to Workout on first tick if we assume manual start handled elsewhere
    // Or we treat Ready as "waiting for start".
    // Assuming the loop only runs when "started", we can treat Ready -> Workout transition immediately?
    // In original code: startWorkout() sets workoutStarted=true, then interval runs statemachine()
    // statemachine checks timers.

    // Logic from original statemachine():
    // if (workoutTimer > 0) -> Workout
    // else if (pauseTimer - 1 > 0 && ...) -> Rest
    // else -> New Set (increment set, reset timers)

    // Handling the "Ready" phase transition to "Workout" for the very first tick?
    if (newState.phase === WorkoutPhase.Ready) {
        // Initial transition
        newState.phase = WorkoutPhase.Workout;
        events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });
        events.push({ type: 'start_exercise', exerciseName: currentExercise.name });
    }

    if (newState.workoutTimer > 0) {
        // In Workout Phase
        if (newState.phase !== WorkoutPhase.Workout) {
            newState.phase = WorkoutPhase.Workout;
            events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });
        }

        // Check for sounds before decrementing (based on original logic order, actually original logic decrements then checks)
        // Original: 
        // 1. Dec/State update
        // 2. updateSound() -> checks CURRENT state/timer

        // Let's decrement first to match "next state" logic
        newState.workoutTimer--;

        // Sound logic (based on original updateSound)
        // STATE_WORKOUT sounds:
        // if (workoutTimer === 0) -> pauseSound
        // if (workoutTimer <= 3 && workoutTimer > 0) -> almostPauseSound
        // intermediate beeps logic

        if (newState.workoutTimer === 0) {
            events.push({ type: 'play_sound', sound: 'pause' });
        } else if (newState.workoutTimer <= 3 && newState.workoutTimer > 0) {
            events.push({ type: 'play_sound', sound: 'almost_pause' });
        }

        // Intermediate beeps
        if (currentExercise.intermediateBeeps) {
            // We need to match the exact timer value.
            // Original logic used an index (intermBeepsIdx) to track progress.
            // Since this is a pure function, we can just check if current timer is in the list.
            if (currentExercise.intermediateBeeps.includes(newState.workoutTimer)) {
                events.push({ type: 'play_sound', sound: 'intermediate' });
            }
        }

    } else if (newState.pauseTimer - 1 > 0 && newState.exerciseIndex < exercises.length - 1) {
        // In Rest Phase
        // Note: The condition `pauseTimer - 1 > 0` implies we stop resting when pauseTimer is 1?
        // Original: `else if (pauseTimer - 1 > 0 && ...)` -> `pauseTimer--; state = STATE_REST`
        // So if pauseTimer is 2, 2-1 > 0 is true, decrement to 1, state=Rest.
        // If pauseTimer is 1, 1-1 > 0 is false, fall through to New Set.

        if (newState.phase !== WorkoutPhase.Rest) {
            newState.phase = WorkoutPhase.Rest;
            events.push({ type: 'phase_change', phase: WorkoutPhase.Rest });
        }

        newState.pauseTimer--;

        // Sound logic (STATE_REST)
        // if (pauseTimer <= 3 && pauseTimer > 0) -> almostStartSound
        if (newState.pauseTimer <= 3 && newState.pauseTimer > 0) {
            events.push({ type: 'play_sound', sound: 'almost_start' });
        }

    } else {
        // New Set / Next Exercise
        newState.setIndex++;

        if (newState.setIndex > currentExercise.setCount) {
            // Next Exercise
            newState.exerciseIndex++;
            newState.setIndex = 1;

            if (newState.exerciseIndex >= exercises.length) {
                newState.phase = WorkoutPhase.Finished;
                events.push({ type: 'finished' });
                return { newState, events };
            }

            // New Exercise setup
            events.push({ type: 'start_exercise', exerciseName: exercises[newState.exerciseIndex].name });
        }

        // Reset timers for new set/exercise
        const nextEx = exercises[newState.exerciseIndex];
        newState.workoutTimer = nextEx.workoutTime;
        newState.pauseTimer = nextEx.pauseTime;

        // This is essentially the "New Set" state start
        newState.phase = WorkoutPhase.Workout;

        // Sound for STATE_NEW_SET in original:
        // if (!pauseState && workoutTimer === exercises[currentExercise].workoutTime) -> startSound
        events.push({ type: 'play_sound', sound: 'start' });
        events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });
    }

    return { newState, events };
}
