import longBeepUrl from './assets/long_beep.mp3';
import almostSoundUrl from './assets/almostSound.mp3';
import shortBeepUrl from './assets/short_beep.mp3';
import intermediateSoundUrl from './assets/intermediateSound.mp3';

export type SoundType = 'start' | 'pause' | 'almost_start' | 'almost_pause' | 'intermediate';

export class AudioManager {
    private sounds = new Map<SoundType, HTMLAudioElement>();

    // Map types to imported URLs
    private readonly soundUrls: Record<SoundType, string> = {
        'start': longBeepUrl,
        'pause': shortBeepUrl,
        'almost_start': almostSoundUrl,
        'almost_pause': almostSoundUrl,
        'intermediate': intermediateSoundUrl,
    };

    /**
     * Preload all audio assets.
     * Returns a promise that resolves when all sounds are loaded (or have failed gracefully).
     */
    public async preload(): Promise<void> {
        const promises = Object.entries(this.soundUrls).map(([type, url]) => {
            return new Promise<void>((resolve) => {
                const audio = new Audio();
                audio.src = url;
                audio.preload = 'auto';

                const cleanup = () => {
                    audio.removeEventListener('canplaythrough', onLoaded);
                    audio.removeEventListener('error', onError);
                };

                const onLoaded = () => {
                    cleanup();
                    this.sounds.set(type as SoundType, audio);
                    resolve();
                };

                const onError = (e: Event | string) => {
                    cleanup();
                    console.warn(`[AudioManager] Failed to load sound '${type}' (${url}):`, e);
                    // Resolve anyway so the app can start even if some sounds fail
                    resolve();
                };

                audio.addEventListener('canplaythrough', onLoaded);
                audio.addEventListener('error', onError);

                // Trigger load
                audio.load();
            });
        });

        await Promise.all(promises);
        console.log(`[AudioManager] Preloaded ${this.sounds.size}/${Object.keys(this.soundUrls).length} sound assets.`);
    }

    /**
     * Play a sound by type.
     * Resets current time to 0 to allow rapid replays.
     */
    public play(sound: SoundType): void {
        const audio = this.sounds.get(sound);
        if (audio) {
            audio.currentTime = 0;
            audio.play().catch(e => console.warn(`Audio play failed for ${sound}:`, e));
        } else {
            console.warn(`[AudioManager] Cannot play '${sound}': Asset not loaded.`);
        }
    }
}
