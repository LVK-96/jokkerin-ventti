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

    it('transitions from Ready to Workout on first tick', () => {
        const state = createInitialState(mockExercises);
        const { newState, events } = tick(state, mockExercises);
        
        expect(newState.phase).toBe(WorkoutPhase.Workout);
        expect(newState.workoutTimer).toBe(4); // Decremented
        expect(events).toContainEqual({ type: 'phase_change', phase: WorkoutPhase.Workout });
        expect(events).toContainEqual({ type: 'start_exercise', exerciseName: 'Ex1' });
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

        // Tick to 0 (pause sound)
        state.workoutTimer = 1;
        result = tick(state, mockExercises);
        state = result.newState;
        expect(state.workoutTimer).toBe(0);
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'pause' });
    });

    it('transitions to Rest when workout timer hits 0', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Workout,
            exerciseIndex: 0,
            setIndex: 1,
            workoutTimer: 0,
            pauseTimer: 3
        };

        // pauseTimer is 3. 3-1 > 0 is true.
        const result = tick(state, mockExercises);
        state = result.newState;
        
        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(state.pauseTimer).toBe(2);
        expect(result.events).toContainEqual({ type: 'phase_change', phase: WorkoutPhase.Rest });
    });

    it('transitions to Next Set after Rest', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Rest,
            exerciseIndex: 0,
            setIndex: 1,
            workoutTimer: 0,
            pauseTimer: 2 // 2-1 > 0 is false? No 1 > 0 is true. 
            // Logic: `pauseTimer - 1 > 0`. If pauseTimer=2, 1>0 True. -> Rest.
            // If pauseTimer=1. 0>0 False -> New Set.
        };
        
        // Tick to 1
        let result = tick(state, mockExercises);
        state = result.newState;
        expect(state.phase).toBe(WorkoutPhase.Rest);
        expect(state.pauseTimer).toBe(1);
        
        // Tick to Next Set
        result = tick(state, mockExercises);
        state = result.newState;
        
        expect(state.phase).toBe(WorkoutPhase.Workout);
        expect(state.setIndex).toBe(2);
        expect(state.workoutTimer).toBe(5); // Reset
        expect(result.events).toContainEqual({ type: 'play_sound', sound: 'start' });
    });

    it('transitions to Next Exercise when sets complete', () => {
        let state: WorkoutState = {
            phase: WorkoutPhase.Rest, // Actually it transitions from "Rest end" to "New Set/Ex"
            exerciseIndex: 0,
            setIndex: 2, // Last set of Ex1
            workoutTimer: 0,
            pauseTimer: 1 // About to finish rest
        };

        const result = tick(state, mockExercises);
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
            pauseTimer: 1
        };

        const result = tick(state, mockExercises);
        state = result.newState;

        expect(state.phase).toBe(WorkoutPhase.Finished);
        expect(result.events).toContainEqual({ type: 'finished' });
    });
});
