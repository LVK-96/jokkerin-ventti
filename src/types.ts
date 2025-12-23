export interface Exercise {
    name: string;
    workoutTime: number;
    pauseTime: number;
    setCount: number;
    intermediateBeeps?: number[];
}

export interface Workout {
    exercises: Exercise[];
}
