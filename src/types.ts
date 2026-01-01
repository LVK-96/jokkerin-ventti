import { AnimationId } from '../wasm/pkg/jokkerin_ventti_wasm';

export interface Exercise {
    name: string;
    animationId?: AnimationId;
    workoutTime: number;
    pauseTime: number;
    setCount: number;
    intermediateBeeps?: number[];
}

export interface Workout {
    exercises: Exercise[];
}
