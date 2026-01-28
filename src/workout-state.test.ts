import { describe, it, expect } from 'vitest';
import { tick, createInitialState, WorkoutPhase, WorkoutState } from './workout-state';
import { Exercise } from './types';

describe('Workout State Machine', () => {
    const mockExercises: Exercise[] = [
        { name: 'Ex1', workoutTime: 5, pauseTime: 3, setCount: 2, intermediateBeeps: [2] },
        { name: 'Ex2', workoutTime: 4, pauseTime: 2, setCount: 1 }
    ];

    it('creates initial state correctly', () => {
        const state = createInitialState(mockExercises);
        expect(state.phase).toBe(WorkoutPhase.Ready);
        expect(state.exerciseIndex).toBe(0);
        expect(state.setIndex).toBe(1);
        expect(state.workoutTimer).toBe(5);
    });

    it('counts down during Ready phase before transitioning to Workout', () => {
        const state = createInitialState(mockExercises);
        // pauseTime (Ready phase duration) is now fixed at 10s

        // First tick: pauseTimer 10 -> 9, still Ready
        let result = tick(state, mockExercises);
        expect(result.newState.phase).toBe(WorkoutPhase.Ready);
        expect(result.newState.pauseTimer).toBe(9);

        // Fast forward to near end (pauseTimer = 2)
        let currentState = result.newState;
        currentState.pauseTimer = 2;

        // Next tick: pauseTimer 2 -> 1, still Ready
        result = tick(currentState, mockExercises);
        expect(result.newState.phase).toBe(WorkoutPhase.Ready);
        expect(result.newState.pauseTimer).toBe(1);
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'almost_start' });

        // Final tick: transitions to Workout (At 1)
        result = tick(result.newState, mockExercises);
        expect(result.newState.phase).toBe(WorkoutPhase.Workout);
        expect(result.events).toContainEqual({ type: 'phase_change', phase: WorkoutPhase.Workout });
        expect(result.events).toContainEqual({ type: 'start_exercise', exerciseName: 'Ex1' });
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'start' });
    });

    it('decrements workout timer and emits sounds', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Workout,
            exerciseIndex: 0,
            setIndex: 1,
            workoutTimer: 4,
            pauseTimer: 3
        };

        // Tick to 3 (almost pause sound range)
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.workoutTimer).toBe(3);
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'almost_pause' });

        // Tick to 2 (intermediate beep)
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.workoutTimer).toBe(2);
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'almost_pause' });
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'intermediate' });

        // Tick to 1 (last second)
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.workoutTimer).toBe(1);
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'almost_pause' });
    });

    it('transitions to Rest when workout timer is 1', () => {
        // Initial state: Workout phase, timer at 1
        let state = createInitialState(mockExercises);
        state.phase = WorkoutPhase.Workout;
        state.workoutTimer = 1;

        // Tick: 1 -> Rest
        const result = tick(state, mockExercises);
        state = result.newState;

        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(result.events).toContainEqual({ type: 'phase_change', phase: WorkoutPhase.Rest });
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'pause' });
    });

    it('transitions to Rest when workout timer is 1', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Workout,
            exerciseIndex: 0,
            setIndex: 1,
            workoutTimer: 2, // 2 -> 1.
            pauseTimer: 3
        };

        // Tick 2 -> 1
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.workoutTimer).toBe(1);

        // Tick 1 -> Rest
        result = tick(state, mockExercises);
        state = result.newState;

        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(state.pauseTimer).toBe(3); // Should NOT decrement on first tick of Rest logic
        expect(result.events).toContainEqual({ type: 'phase_change', phase: WorkoutPhase.Rest });
    });

    it('transitions to Next Set after Rest', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Workout, // Start in Workout so we transition to Rest
            exerciseIndex: 0,
            setIndex: 1,
            workoutTimer: 0, // Force transition
            pauseTimer: 2
        };


        // Tick to Rest (Initial entry)
        // logic: if workoutTimer > 1 (0 is not > 1). Transition to Rest.
        // Transition: returns Rest state, pauseTimer 2.
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(state.pauseTimer).toBe(2); // Initial value held

        // Tick 2 -> 1
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.pauseTimer).toBe(1); // Normal decrement

        // Tick 1 -> Transition to Next Set
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Workout);
        expect(state.setIndex).toBe(2);
        expect(state.workoutTimer).toBe(5); // Reset
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'start' });
    });

    it('transitions to Next Exercise when sets complete', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Rest,
            exerciseIndex: 0,
            setIndex: 2, // Last set of Ex1
            workoutTimer: 0,
            pauseTimer: 2
        };

        // Tick 2 -> 1
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(state.pauseTimer).toBe(1);

        // Tick 1 -> Transition
        result = tick(state, mockExercises);
        state = result.newState;

        expect(state.exerciseIndex).toBe(1);
        expect(state.setIndex).toBe(1);
        expect(state.workoutTimer).toBe(4); // Ex2 time
        expect(result.events).toContainEqual({ type: 'start_exercise', exerciseName: 'Ex2' });
    });

    it('finishes workout', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Rest,
            exerciseIndex: 1, // Ex2 has 1 set
            setIndex: 1,
            workoutTimer: 0,
            pauseTimer: 2
        };

        // Should transition immediately to Finished because we shouldn't rest at end of workout
        const result = tick(state, mockExercises);
        state = result.newState;

        expect(state.phase).toBe(WorkoutPhase.Finished);
        expect(result.events).toContainEqual({ type: 'finished' });
    });

    it('bug reproduction: ensures first rest is not skipped', () => {
        // Initial state: Ready phase
        let state = createInitialState(mockExercises);
        // Ex1 has pauseTime: 3

        // Fast forward Ready phase to 1
        state.pauseTimer = 1;

        // Tick 1: Ready -> Workout
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Workout);

        // Verify pauseTimer is reset to Ex1.pauseTime (3)
        // This is crucial: if it remains 1, the subsequent Rest phase will be skipped
        expect(state.pauseTimer).toBe(3); 

        // Fast forward Workout to 1 (end of workout)
        state.workoutTimer = 1;

        // Tick 2: Workout -> Rest
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Rest);

        // Tick 3: Rest logic
        // Should NOT skip rest immediately
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Rest);
    });
});
