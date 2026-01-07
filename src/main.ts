import { WorkoutApp } from './workout-app';

async function bootstrap() {
    const app = new WorkoutApp();

    // Exposed for debugging
    (window as any).workoutApp = app;

    try {
        await app.init();
    } catch (e) {
        console.error("Fatal Application Error:", e);
    }
}

bootstrap();
