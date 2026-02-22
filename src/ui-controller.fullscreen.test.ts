import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { UIController } from './ui-controller';

function setupDom() {
    document.body.innerHTML = `
        <button id="startButton">Start</button>
        <div id="progress-bar-container"></div>
        <div id="progress-bar"></div>
        <div id="timer"></div>
        <button id="prevButton"></button>
        <button id="nextButton"></button>
        <div id="exercise-count"></div>
        <div id="exercise-name"></div>
        <div id="set-count"></div>
        <input id="textSizeSlider" type="range" value="25" />
    `;
}

describe('UIController fullscreen toggle', () => {
    beforeEach(() => {
        setupDom();
    });

    afterEach(() => {
        document.body.innerHTML = '';
        vi.restoreAllMocks();
    });

    it('requests fullscreen when fullscreen is not active', async () => {
        const requestFullscreen = vi.fn().mockResolvedValue(undefined);
        Object.defineProperty(document.documentElement, 'requestFullscreen', {
            configurable: true,
            value: requestFullscreen
        });

        Object.defineProperty(document, 'fullscreenElement', {
            configurable: true,
            get: () => null
        });

        const ui = new UIController();
        ui.createFullscreenButton();

        const button = document.getElementById('fullscreen-button');
        expect(button).not.toBeNull();

        button?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        await Promise.resolve();

        expect(requestFullscreen).toHaveBeenCalledTimes(1);
    });

    it('exits fullscreen when already active', async () => {
        const activeElement = document.createElement('div');

        Object.defineProperty(document, 'fullscreenElement', {
            configurable: true,
            get: () => activeElement
        });

        const exitFullscreen = vi.fn().mockResolvedValue(undefined);
        Object.defineProperty(document, 'exitFullscreen', {
            configurable: true,
            value: exitFullscreen
        });

        const ui = new UIController();
        ui.createFullscreenButton();

        const button = document.getElementById('fullscreen-button');
        expect(button).not.toBeNull();

        button?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        await Promise.resolve();

        expect(exitFullscreen).toHaveBeenCalledTimes(1);
    });
});
