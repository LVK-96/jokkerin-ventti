import longBeepUrl from './assets/long_beep.mp3';
import almostSoundUrl from './assets/almostSound.mp3';
import shortBeepUrl from './assets/short_beep.mp3';
import intermediateSoundUrl from './assets/intermediateSound.mp3';

export type SoundType = 'start' | 'pause' | 'almost_start' | 'almost_pause' | 'intermediate';

export class AudioManager {
    private sounds = new Map<SoundType, HTMLAudioElement>([
        ['start', new Audio(longBeepUrl)],
        ['pause', new Audio(shortBeepUrl)],
        ['almost_start', new Audio(almostSoundUrl)],
        ['almost_pause', new Audio(almostSoundUrl)],
        ['intermediate', new Audio(intermediateSoundUrl)],
    ]);

    /**
     * Play a sound by type.
     * Resets current time to 0 to allow rapid replays.
     */
    public play(sound: SoundType): void {
        const audio = this.sounds.get(sound);
        if (audio) {
            audio.currentTime = 0;
            audio.play().catch(e => console.warn(`Audio play failed for ${sound}:`, e));
        }
    }
}
