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

const READY_PHASE_DURATION = 10; // seconds before first exercise starts

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
        pauseTimer: READY_PHASE_DURATION
    };
}

/**
 * Pure function to calculate the next state based on current state and exercises.
 * Returns the new state and any events that occurred during the transition.
 */
interface TickResult {
    newState: WorkoutState;
    events: WorkoutEvent[];
}

export function tick(state: WorkoutState, exercises: Exercise[]): TickResult {
    switch (state.phase) {
        case WorkoutPhase.Ready:
            return handleReadyTick(state, exercises);
        case WorkoutPhase.Workout:
            return handleWorkoutTick(state, exercises);
        case WorkoutPhase.Rest:
            return handleRestTick(state, exercises);
        case WorkoutPhase.Finished:
            return handleFinishedTick(state);
        default:
            return { newState: state, events: [] };
    }
}

function handleReadyTick(state: WorkoutState, exercises: Exercise[]): TickResult {
    const newState = { ...state };
    const events: WorkoutEvent[] = [];
    const currentExercise = exercises[state.exerciseIndex];

    if (newState.pauseTimer > 1) {
        // Countdown in progress
        newState.pauseTimer--;

        // Sound logic
        if (newState.pauseTimer <= 3 && newState.pauseTimer > 0) {
            events.push({ type: 'play_sound', sound: 'almost_start' });
        }
        return { newState, events };
    } else {
        // Countdown finished - transition to Workout
        newState.phase = WorkoutPhase.Workout;
        events.push({ type: 'play_sound', sound: 'start' });
        events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });
        events.push({ type: 'start_exercise', exerciseName: currentExercise.name });
        return { newState, events };
    }
}

function handleWorkoutTick(state: WorkoutState, exercises: Exercise[]): TickResult {
    const newState = { ...state };
    const events: WorkoutEvent[] = [];
    const currentExercise = exercises[state.exerciseIndex];

    // Ensure we are in correct phase object-wise
    if (newState.phase !== WorkoutPhase.Workout) {
        newState.phase = WorkoutPhase.Workout;
        events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });
    }

    if (newState.workoutTimer > 1) {
        newState.workoutTimer--;

        // Sounds
        if (newState.workoutTimer === 0) {
            events.push({ type: 'play_sound', sound: 'pause' });
        } else if (newState.workoutTimer <= 3 && newState.workoutTimer > 0) {
            events.push({ type: 'play_sound', sound: 'almost_pause' });
        }

        // Intermediate beeps
        if (currentExercise.intermediateBeeps && currentExercise.intermediateBeeps.includes(newState.workoutTimer)) {
            events.push({ type: 'play_sound', sound: 'intermediate' });
        }

        return { newState, events };
    } else {
        // Workout finished, determine next step (Rest or Next Round)

        // Check if we should rest
        // We rest if NOT at the very end of the workout (last set of last exercise).
        const isLastSetOfLastExample = (newState.exerciseIndex === exercises.length - 1 && newState.setIndex === currentExercise.setCount);

        if (!isLastSetOfLastExample) {
            return transitionToRest(newState);
        } else {
            return advanceToNextRound(newState, exercises);
        }
    }
}

function transitionToRest(state: WorkoutState): TickResult {
    const newState = { ...state };
    const events: WorkoutEvent[] = [];

    newState.phase = WorkoutPhase.Rest;
    events.push({ type: 'phase_change', phase: WorkoutPhase.Rest });
    return { newState, events };
}

function handleRestTick(state: WorkoutState, exercises: Exercise[]): TickResult {
    const newState = { ...state };
    const events: WorkoutEvent[] = [];
    const currentExercise = exercises[state.exerciseIndex];

    // Ensure state object matches
    if (newState.phase !== WorkoutPhase.Rest) {
        // Should catch this in transition, but for safety:
        newState.phase = WorkoutPhase.Rest;
        events.push({ type: 'phase_change', phase: WorkoutPhase.Rest });
        // If we force transition here, we should return to skip decrement?
        // But main switch dispatches by phase. So we are likely already in Rest phase property-wise.
    }

    const isLastSetOfLastExample = (newState.exerciseIndex === exercises.length - 1 && newState.setIndex === currentExercise.setCount);

    if (newState.pauseTimer > 1 && !isLastSetOfLastExample) { // Keep > 1 as per user request
        newState.pauseTimer--;

        if (newState.pauseTimer <= 3 && newState.pauseTimer > 0) {
            events.push({ type: 'play_sound', sound: 'almost_start' });
        }
        return { newState, events };
    } else {
        // Rest finished or skipped (invalid rest state)
        return advanceToNextRound(newState, exercises);
    }
}

function handleFinishedTick(state: WorkoutState): TickResult {
    return { newState: state, events: [] };
}


function advanceToNextRound(state: WorkoutState, exercises: Exercise[]): TickResult {
    const newState = { ...state };
    const events: WorkoutEvent[] = [];
    const currentExercise = exercises[state.exerciseIndex];

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

    newState.phase = WorkoutPhase.Workout;
    events.push({ type: 'play_sound', sound: 'start' });
    events.push({ type: 'phase_change', phase: WorkoutPhase.Workout });

    return { newState, events };
}
