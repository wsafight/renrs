export {};

declare global {
  interface Window {
    blockedAudio: boolean;
    readWrites: number;
    startedAudio: boolean;
    testAudio: HTMLAudioElement[];
  }
}
